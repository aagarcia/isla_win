//! La ventana de la isla: siempre encima, sin foco, fuera de Alt-Tab, pegada al borde superior y centrada.
//! Decide el tamaño (compacta/expandida) y cuándo repintar; qué dibujar lo decide la `Vista` activa.

use super::escala::Escala;
use super::gancho::{self, WM_CLIC_FUERA};
use super::gdi::pintar_con_doble_buffer;
use super::tema::{COMPACTO, EXPANDIDO, INTERVALO_ANIMACION_MS, RADIO_ISLA};
use super::vista;
use crate::estado;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Mensaje a la ventana: cambió lo que suena y hay que reaccionar.
const WM_ESTADO_CAMBIADO: u32 = WM_APP;
const ID_TIMER_ANIMACION: usize = 1;

static EXPANDIDA: AtomicBool = AtomicBool::new(false);
/// El contenido compacto no cabe y se desplaza: hace falta repintar aunque no suene nada.
static DESBORDA: AtomicBool = AtomicBool::new(false);
static TIMER_ACTIVO: AtomicBool = AtomicBool::new(false);

pub fn crear() -> windows::core::Result<HWND> {
    unsafe {
        let instancia = GetModuleHandleW(None)?;
        let clase = w!("IslaWin");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(procedimiento),
            // sin cursor de clase Windows deja el de "ocupado" (el círculo azul)
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: instancia.into(),
            lpszClassName: clase,
            ..Default::default()
        });
        let (ancho, alto) = dimensiones(false);
        let ventana = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            clase, PCWSTR::null(), WS_POPUP, 0, 0, ancho, alto, None, None, Some(instancia.into()), None,
        )?;
        let _ = SetLayeredWindowAttributes(ventana, COLORREF(0), 255, LWA_ALPHA);
        aplicar_tamano(ventana);
        Ok(ventana)
    }
}

/// Avisa a la ventana de que cambió el estado. Se puede llamar desde cualquier hilo.
pub fn avisar(ventana: isize) {
    unsafe {
        let _ = PostMessageW(Some(HWND(ventana as *mut _)), WM_ESTADO_CAMBIADO, WPARAM(0), LPARAM(0));
    }
}

pub fn bucle_de_mensajes() {
    unsafe {
        let mut mensaje = MSG::default();
        while GetMessageW(&mut mensaje, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&mensaje);
            DispatchMessageW(&mensaje);
        }
    }
}

unsafe extern "system" fn procedimiento(ventana: HWND, mensaje: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match mensaje {
            WM_ESTADO_CAMBIADO => al_cambiar_estado(ventana),
            WM_CLIC_FUERA => al_clic_fuera(ventana),
            WM_TIMER => al_temporizador(ventana),
            WM_LBUTTONDOWN => al_clic(ventana, coordenadas(lparam)),
            WM_SETCURSOR => al_poner_cursor(ventana),
            WM_PAINT => al_pintar(ventana),
            WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize), // clic sin robar el foco
            WM_ERASEBKGND => return LRESULT(1),                          // se pinta todo, sin parpadeo
            WM_DESTROY => {
                gancho::desactivar();
                PostQuitMessage(0);
            }
            _ => return DefWindowProcW(ventana, mensaje, wparam, lparam),
        }
        LRESULT(0)
    }
}

// ---------- reacciones a los mensajes ----------

unsafe fn al_cambiar_estado(ventana: HWND) {
    unsafe {
        let hay_pista = estado::pista().is_some();
        if !hay_pista {
            EXPANDIDA.store(false, Ordering::Relaxed);
            gancho::desactivar();
            aplicar_tamano(ventana);
        }
        let _ = ShowWindow(ventana, if hay_pista { SW_SHOWNOACTIVATE } else { SW_HIDE });
        if hay_pista {
            let _ = InvalidateRect(Some(ventana), None, false);
        }
        ajustar_temporizador(ventana);
    }
}

unsafe fn al_clic(ventana: HWND, punto: POINT) {
    unsafe {
        if !EXPANDIDA.load(Ordering::Relaxed) {
            return cambiar_expansion(ventana, true);
        }
        let boton_pulsado = vista::activa().is_some_and(|v| v.pulsar(punto, area_cliente(ventana), Escala::actual()));
        if !boton_pulsado {
            cambiar_expansion(ventana, false); // un clic en otra zona de la isla la compacta
        }
    }
}

unsafe fn al_clic_fuera(ventana: HWND) {
    unsafe {
        if EXPANDIDA.load(Ordering::Relaxed) {
            cambiar_expansion(ventana, false);
        }
    }
}

unsafe fn al_temporizador(ventana: HWND) {
    unsafe {
        let _ = InvalidateRect(Some(ventana), None, false);
    }
}

/// Manita donde un clic hace algo: la isla compacta (expande) y los botones del estado expandido.
unsafe fn al_poner_cursor(ventana: HWND) {
    unsafe {
        let mut punto = POINT::default();
        let _ = GetCursorPos(&mut punto);
        let _ = ScreenToClient(ventana, &mut punto);
        let sobre_boton = || vista::activa().is_some_and(|v| v.hay_boton_en(punto, area_cliente(ventana), Escala::actual()));
        let manita = !EXPANDIDA.load(Ordering::Relaxed) || sobre_boton();
        SetCursor(LoadCursorW(None, if manita { IDC_HAND } else { IDC_ARROW }).ok());
    }
}

unsafe fn al_pintar(ventana: HWND) {
    unsafe {
        let mut pintura = PAINTSTRUCT::default();
        let hdc = BeginPaint(ventana, &mut pintura);
        let area = area_cliente(ventana);
        let expandida = EXPANDIDA.load(Ordering::Relaxed);
        let desborda = pintar_con_doble_buffer(hdc, area.right, area.bottom, Escala::actual(), |lienzo| {
            match vista::activa() {
                Some(v) if expandida => {
                    v.expandida(lienzo, area);
                    false
                }
                Some(v) => v.compacta(lienzo, area),
                None => false,
            }
        });
        DESBORDA.store(desborda, Ordering::Relaxed);
        let _ = EndPaint(ventana, &pintura);
        ajustar_temporizador(ventana);
    }
}

// ---------- tamaño, forma y temporizador ----------

fn dimensiones(expandida: bool) -> (i32, i32) {
    let (ancho, alto) = if expandida { EXPANDIDO } else { COMPACTO };
    let escala = Escala::actual();
    (escala.px(ancho), escala.px(alto))
}

/// Ajusta tamaño y forma según el estado; sigue centrada y pegada arriba.
unsafe fn aplicar_tamano(ventana: HWND) {
    unsafe {
        let (ancho, alto) = dimensiones(EXPANDIDA.load(Ordering::Relaxed));
        let x = (GetSystemMetrics(SM_CXSCREEN) - ancho) / 2;
        let _ = SetWindowPos(ventana, None, x, 0, ancho, alto, SWP_NOACTIVATE | SWP_NOZORDER);
        // esquinas superiores rectas: el redondeo sube por encima del borde de la pantalla
        let radio = Escala::actual().px(RADIO_ISLA);
        SetWindowRgn(ventana, Some(CreateRoundRectRgn(0, -radio, ancho + 1, alto + 1, 2 * radio, 2 * radio)), true);
    }
}

unsafe fn cambiar_expansion(ventana: HWND, expandir: bool) {
    unsafe {
        EXPANDIDA.store(expandir, Ordering::Relaxed);
        if expandir {
            gancho::activar(ventana);
        } else {
            gancho::desactivar();
        }
        aplicar_tamano(ventana);
        let _ = InvalidateRect(Some(ventana), None, false);
    }
}

/// El temporizador solo corre mientras algo se anima; en reposo no gasta CPU.
unsafe fn ajustar_temporizador(ventana: HWND) {
    unsafe {
        let animar = vista::activa().is_some_and(|v| v.animando()) || DESBORDA.load(Ordering::Relaxed);
        if TIMER_ACTIVO.swap(animar, Ordering::Relaxed) == animar {
            return;
        }
        if animar {
            SetTimer(Some(ventana), ID_TIMER_ANIMACION, INTERVALO_ANIMACION_MS, None);
        } else {
            let _ = KillTimer(Some(ventana), ID_TIMER_ANIMACION);
        }
    }
}

// ---------- utilidades ----------

unsafe fn area_cliente(ventana: HWND) -> RECT {
    let mut area = RECT::default();
    unsafe {
        let _ = GetClientRect(ventana, &mut area);
    }
    area
}

/// Coordenadas (con signo) que Windows empaqueta en el `lparam` de los mensajes de ratón.
fn coordenadas(lparam: LPARAM) -> POINT {
    POINT { x: (lparam.0 & 0xFFFF) as i16 as i32, y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32 }
}
