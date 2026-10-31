//! Icono junto al reloj (área de notificación) y su menú: ocultar/mostrar, tamaño en reposo y cerrar.
//! No decide nada: traduce los clics del usuario a `Orden` y la ventana las ejecuta.

use super::textos::{actuales, ancho, idioma, Idioma};
use crate::dominio::preferencias::{Preferencias, Reposo};
use std::ffi::c_void;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Mensaje que Windows envía a la ventana cuando el usuario toca el icono (en `lparam`, el evento del ratón).
pub const WM_BANDEJA: u32 = WM_APP + 2;

const ID_ICONO: u32 = 1;
const TEXTO_AYUDA: &str = "Isla";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Orden {
    AlternarVisibilidad,
    Reposo(Reposo),
    Idioma(Idioma),
    Cerrar,
}

const ID_ALTERNAR: usize = 1;
const ID_MINIMAL: usize = 2;
const ID_COMPACT: usize = 3;
const ID_CERRAR: usize = 4;
const ID_ESPANOL: usize = 5;
const ID_INGLES: usize = 6;

impl Orden {
    /// Orden asociada al identificador de un elemento del menú.
    pub fn desde_id(id: usize) -> Option<Self> {
        match id {
            ID_ALTERNAR => Some(Self::AlternarVisibilidad),
            ID_MINIMAL => Some(Self::Reposo(Reposo::Minimal)),
            ID_COMPACT => Some(Self::Reposo(Reposo::Compact)),
            ID_ESPANOL => Some(Self::Idioma(Idioma::Es)),
            ID_INGLES => Some(Self::Idioma(Idioma::En)),
            ID_CERRAR => Some(Self::Cerrar),
            _ => None,
        }
    }
}

/// Evento del ratón que Windows empaqueta en el `lparam` del mensaje de la bandeja.
pub fn evento_de_raton(lparam: LPARAM) -> u32 {
    (lparam.0 & 0xFFFF) as u32
}

pub fn agregar(ventana: HWND) -> windows::core::Result<()> {
    let mut datos = datos(ventana);
    datos.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    datos.uCallbackMessage = WM_BANDEJA;
    datos.hIcon = crear_icono()?;
    let ayuda: Vec<u16> = TEXTO_AYUDA.encode_utf16().collect();
    datos.szTip[..ayuda.len()].copy_from_slice(&ayuda);
    unsafe { Shell_NotifyIconW(NIM_ADD, &datos).ok() }
}

pub fn quitar(ventana: HWND) {
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &datos(ventana));
    }
}

fn datos(ventana: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: ventana, uID: ID_ICONO, ..Default::default() }
}

/// Muestra el menú junto al cursor. Cada elemento llega a la ventana como `WM_COMMAND` con su id.
pub fn mostrar_menu(ventana: HWND, preferencias: &Preferencias) {
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return };
        let textos = actuales();
        let alternar = ancho(if preferencias.oculta_por_usuario { textos.mostrar } else { textos.ocultar });
        let _ = AppendMenuW(menu, MF_STRING, ID_ALTERNAR, PCWSTR(alternar.as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, PCWSTR(ancho(textos.reposo).as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, ID_MINIMAL, w!("Minimal"));
        let _ = AppendMenuW(menu, MF_STRING, ID_COMPACT, w!("Compact"));
        let marcado = if preferencias.reposo == Reposo::Minimal { ID_MINIMAL } else { ID_COMPACT };
        let _ = CheckMenuRadioItem(menu, ID_MINIMAL as u32, ID_COMPACT as u32, marcado as u32, MF_BYCOMMAND.0);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, PCWSTR(ancho(textos.idioma).as_ptr()));
        // cada idioma con su propio nombre, para poder volver a encontrarlo si el menú quedó en otro
        let _ = AppendMenuW(menu, MF_STRING, ID_ESPANOL, w!("Español"));
        let _ = AppendMenuW(menu, MF_STRING, ID_INGLES, w!("English"));
        let marcado = if idioma() == Idioma::Es { ID_ESPANOL } else { ID_INGLES };
        let _ = CheckMenuRadioItem(menu, ID_ESPANOL as u32, ID_INGLES as u32, marcado as u32, MF_BYCOMMAND.0);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_CERRAR, PCWSTR(ancho(textos.cerrar).as_ptr()));

        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        // sin pasar al primer plano el menú no se cierra al hacer clic fuera de él
        let _ = SetForegroundWindow(ventana);
        let _ = TrackPopupMenu(menu, TPM_RIGHTALIGN | TPM_BOTTOMALIGN, cursor.x, cursor.y, None, ventana, None);
        let _ = PostMessageW(Some(ventana), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
    }
}

// ---------- el icono: una píldora negra con borde blanco y una onda verde ----------

const LADO_ICONO: i32 = 32;
const BLANCO: (u8, u8, u8) = (255, 255, 255);
const NEGRO: (u8, u8, u8) = (0, 0, 0);
const VERDE: (u8, u8, u8) = (30, 215, 96);
/// Grosor del borde blanco: se ve igual sobre una barra de tareas clara u oscura.
const BORDE: f32 = 2.0;
/// (centro x, alto) de cada barra de la onda.
const BARRAS: [(f32, f32); 5] = [(9.0, 5.0), (12.5, 9.0), (16.0, 12.0), (19.5, 8.0), (23.0, 5.0)];

/// Distancia con signo al borde de la píldora (negativa dentro).
fn distancia_a_la_pildora(x: f32, y: f32) -> f32 {
    let (mitad_ancho, mitad_alto) = (14.0, 7.5);
    let radio = mitad_alto;
    let (qx, qy) = ((x - 16.0).abs() - (mitad_ancho - radio), (y - 16.0).abs() - (mitad_alto - radio));
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radio
}

fn es_onda(x: f32, y: f32) -> bool {
    BARRAS.iter().any(|&(cx, alto)| (x - cx).abs() < 1.0 && (y - 16.0).abs() < alto / 2.0)
}

/// Píxeles BGRA del icono (los transparentes quedan en cero).
fn pixeles_icono() -> Vec<u8> {
    let mut bgra = vec![0u8; (LADO_ICONO * LADO_ICONO * 4) as usize];
    for y in 0..LADO_ICONO {
        for x in 0..LADO_ICONO {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let distancia = distancia_a_la_pildora(px, py);
            let color = if distancia > 0.0 {
                continue;
            } else if distancia > -BORDE {
                BLANCO
            } else if es_onda(px, py) {
                VERDE
            } else {
                NEGRO
            };
            let i = ((y * LADO_ICONO + x) * 4) as usize;
            bgra[i..i + 4].copy_from_slice(&[color.2, color.1, color.0, 255]);
        }
    }
    bgra
}

fn crear_icono() -> windows::core::Result<HICON> {
    let bgra = pixeles_icono();
    unsafe {
        let color = CreateBitmap(LADO_ICONO, LADO_ICONO, 1, 32, Some(bgra.as_ptr() as *const c_void));
        let mascara = CreateBitmap(LADO_ICONO, LADO_ICONO, 1, 1, None); // con alfa, la máscara no se usa
        let info = ICONINFO { fIcon: true.into(), xHotspot: 0, yHotspot: 0, hbmMask: mascara, hbmColor: color };
        let icono = CreateIconIndirect(&info);
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mascara.into());
        icono
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn pixel(bgra: &[u8], x: i32, y: i32) -> &[u8] {
        let i = ((y * LADO_ICONO + x) * 4) as usize;
        &bgra[i..i + 4]
    }

    #[test]
    fn el_icono_tiene_el_tamano_esperado() {
        assert_eq!(pixeles_icono().len(), (LADO_ICONO * LADO_ICONO * 4) as usize);
    }

    #[test]
    fn las_esquinas_son_transparentes_y_la_onda_es_verde() {
        let bgra = pixeles_icono();
        assert_eq!(pixel(&bgra, 0, 0)[3], 0);
        assert_eq!(pixel(&bgra, 16, 16), [VERDE.2, VERDE.1, VERDE.0, 255]);
    }

    #[test]
    fn el_borde_es_blanco() {
        let bgra = pixeles_icono();
        assert_eq!(pixel(&bgra, 16, 9), [255, 255, 255, 255]);
    }

    #[test]
    fn cada_id_del_menu_tiene_su_orden() {
        assert_eq!(Orden::desde_id(ID_ALTERNAR), Some(Orden::AlternarVisibilidad));
        assert_eq!(Orden::desde_id(ID_MINIMAL), Some(Orden::Reposo(Reposo::Minimal)));
        assert_eq!(Orden::desde_id(ID_COMPACT), Some(Orden::Reposo(Reposo::Compact)));
        assert_eq!(Orden::desde_id(ID_ESPANOL), Some(Orden::Idioma(Idioma::Es)));
        assert_eq!(Orden::desde_id(ID_INGLES), Some(Orden::Idioma(Idioma::En)));
        assert_eq!(Orden::desde_id(ID_CERRAR), Some(Orden::Cerrar));
        assert_eq!(Orden::desde_id(99), None);
    }
}
