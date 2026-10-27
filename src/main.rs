#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Isla pegada al borde superior con música de Windows (SMTC): Compact y Expanded con controles.
//! Ventana sin foco y siempre encima. Sin sondeo: todo por eventos; el temporizador solo corre
//! mientras hay algo que animar.
//! shortcut: bordes sin antialiasing (GDI), mejorar con DirectComposition si se adopta.
//! shortcut: las suscripciones SMTC se acumulan al cambiar de sesión, des-suscribir si se adopta.

mod media;

use media::{Accion, Pista, INICIO, PISTA};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const COMPACTO: (i32, i32) = (270, 42); // px a 96 dpi, igual que el mockup
const EXPANDIDO: (i32, i32) = (420, 150);
const RADIO: i32 = 22;

static DPI: Mutex<i32> = Mutex::new(96);
static EXPANDIDA: AtomicBool = AtomicBool::new(false);
static DESBORDA: AtomicBool = AtomicBool::new(false); // el título compacto no cabe: se desplaza
static TIMER: AtomicBool = AtomicBool::new(false);
static GANCHO: AtomicIsize = AtomicIsize::new(0); // HHOOK del gancho de ratón (0 = sin instalar)
static VENTANA: AtomicIsize = AtomicIsize::new(0);
const WM_COLAPSAR: u32 = WM_APP + 1;

fn esc(v: i32) -> i32 {
    v * *DPI.lock().unwrap() / 96
}

fn rgb(r: u32, g: u32, b: u32) -> COLORREF {
    COLORREF(r | g << 8 | b << 16)
}

/// Desplazamiento (px) del texto largo: pausa, avanza despacio, pausa y vuelve.
fn desplazamiento(dist: i32, ms: i64) -> i32 {
    let (pausa, vel) = (1500, esc(30) as i64); // vel en px/s
    let dist = dist as i64;
    let mov = dist * 1000 / vel;
    let t = ms % (2 * (pausa + mov));
    let off = if t < pausa { 0 }
        else if t < pausa + mov { (t - pausa) * vel / 1000 }
        else if t < 2 * pausa + mov { dist }
        else { dist - (t - 2 * pausa - mov) * vel / 1000 };
    off.clamp(0, dist) as i32
}

// ---------- dibujo (GDI sobre un DC en memoria) ----------

unsafe fn rect_lleno(hdc: HDC, r: RECT, color: COLORREF, radio: i32) {
    unsafe {
        let b = CreateSolidBrush(color);
        let vb = SelectObject(hdc, b.into());
        let vp = SelectObject(hdc, GetStockObject(NULL_PEN));
        let _ = RoundRect(hdc, r.left, r.top, r.right + 1, r.bottom + 1, radio * 2, radio * 2);
        SelectObject(hdc, vp);
        SelectObject(hdc, vb);
        let _ = DeleteObject(b.into());
    }
}

unsafe fn poligono(hdc: HDC, pts: &[POINT], color: COLORREF) {
    unsafe {
        let b = CreateSolidBrush(color);
        let vb = SelectObject(hdc, b.into());
        let vp = SelectObject(hdc, GetStockObject(NULL_PEN));
        let _ = Polygon(hdc, pts);
        SelectObject(hdc, vp);
        SelectObject(hdc, vb);
        let _ = DeleteObject(b.into());
    }
}

unsafe fn texto(hdc: HDC, s: &str, mut r: RECT, px: i32, peso: i32, color: COLORREF, extra: DRAW_TEXT_FORMAT) {
    unsafe {
        let f = CreateFontW(
            -esc(px), 0, 0, 0, peso, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, 0, w!("Segoe UI Variable Text"),
        );
        let vieja = SelectObject(hdc, f.into());
        SetTextColor(hdc, color);
        let mut t: Vec<u16> = s.encode_utf16().collect();
        if !t.is_empty() {
            DrawTextW(hdc, &mut t, &mut r, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | extra);
        }
        SelectObject(hdc, vieja);
        let _ = DeleteObject(f.into());
    }
}

unsafe fn medir(hdc: HDC, s: &str, px: i32, peso: i32) -> i32 {
    unsafe {
        let f = CreateFontW(
            -esc(px), 0, 0, 0, peso, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, 0, w!("Segoe UI Variable Text"),
        );
        let vieja = SelectObject(hdc, f.into());
        let mut t: Vec<u16> = s.encode_utf16().collect();
        let mut m = RECT::default();
        if !t.is_empty() {
            DrawTextW(hdc, &mut t, &mut m, DT_SINGLELINE | DT_NOPREFIX | DT_CALCRECT);
        }
        SelectObject(hdc, vieja);
        let _ = DeleteObject(f.into());
        m.right - m.left
    }
}

/// Cinco barras del color de la app que bailan mientras suena; quietas y bajas en pausa.
unsafe fn ondas(hdc: HDC, derecha: i32, centro_y: i32, suena: bool, t: f64, color: COLORREF) {
    unsafe {
        let (ancho, sep) = (esc(4), esc(3));
        let x0 = derecha - (5 * ancho + 4 * sep);
        for i in 0..5 {
            let alto = if suena {
                esc(5) + ((esc(15) as f64) * (0.5 + 0.5 * (t * 6.0 + i as f64 * 1.3).sin())) as i32
            } else {
                esc(5)
            };
            let x = x0 + i * (ancho + sep);
            let r = RECT { left: x, top: centro_y - alto / 2, right: x + ancho, bottom: centro_y + alto / 2 };
            rect_lleno(hdc, r, color, esc(2));
        }
    }
}

unsafe fn portada(hdc: HDC, p: &Pista, r: RECT, radio: i32) {
    unsafe {
        let Some(img) = &p.portada else {
            // sin portada (SMTC no la entrega): relleno neutro con una nota, no parece un fallo de carga
            rect_lleno(hdc, r, rgb(48, 48, 56), radio);
            let px = (r.bottom - r.top) * 96 / *DPI.lock().unwrap() * 3 / 5;
            texto(hdc, "♪", r, px, 400, rgb(150, 150, 160), DT_CENTER);
            return;
        };
        let rgn = CreateRoundRectRgn(r.left, r.top, r.right + 1, r.bottom + 1, radio * 2, radio * 2);
        SelectClipRgn(hdc, Some(rgn));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: img.lado,
                biHeight: -img.lado, // de arriba abajo
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        SetStretchBltMode(hdc, HALFTONE);
        StretchDIBits(
            hdc, r.left, r.top, r.right - r.left, r.bottom - r.top, 0, 0, img.lado, img.lado,
            Some(img.bgra.as_ptr() as *const c_void), &info, DIB_RGB_COLORS, SRCCOPY,
        );
        SelectClipRgn(hdc, None);
        let _ = DeleteObject(rgn.into());
    }
}

fn punto(x: i32, y: i32) -> POINT {
    POINT { x, y }
}

#[derive(Clone, Copy, PartialEq)]
enum Icono {
    Anterior,
    Siguiente,
    Play,
    Pausa,
}

unsafe fn icono(hdc: HDC, k: Icono, cx: i32, cy: i32, s: i32, color: COLORREF) {
    unsafe {
        let r = |l, t, rr, b| RECT { left: l, top: t, right: rr, bottom: b };
        match k {
            Icono::Anterior => {
                rect_lleno(hdc, r(cx - s, cy - s, cx - s + s / 3, cy + s), color, 0);
                poligono(hdc, &[punto(cx + s, cy - s), punto(cx + s, cy + s), punto(cx - s + s / 2, cy)], color);
            }
            Icono::Siguiente => {
                rect_lleno(hdc, r(cx + s - s / 3, cy - s, cx + s, cy + s), color, 0);
                poligono(hdc, &[punto(cx - s, cy - s), punto(cx - s, cy + s), punto(cx + s - s / 2, cy)], color);
            }
            Icono::Play => {
                poligono(hdc, &[punto(cx - s * 2 / 3, cy - s), punto(cx - s * 2 / 3, cy + s), punto(cx + s, cy)], color);
            }
            Icono::Pausa => {
                let (a, g) = (s * 2 / 3, s / 2);
                rect_lleno(hdc, r(cx - g / 2 - a, cy - s, cx - g / 2, cy + s), color, 0);
                rect_lleno(hdc, r(cx + g / 2, cy - s, cx + g / 2 + a, cy + s), color, 0);
            }
        }
    }
}

/// Zonas pulsables de la isla expandida: anterior, play/pausa, siguiente.
fn botones(ancho: i32) -> [RECT; 3] {
    let (lado, sep, cy) = (esc(40), esc(72), esc(112));
    let c = |cx: i32| RECT { left: cx - lado / 2, top: cy - lado / 2, right: cx + lado / 2, bottom: cy + lado / 2 };
    [c(ancho / 2 - sep), c(ancho / 2), c(ancho / 2 + sep)]
}

unsafe fn pintar(mem: HDC, r: RECT, p: &Pista, expandida: bool) {
    unsafe {
        let t = p.desde.elapsed().as_secs_f64(); // fase de la onda
        let blanco = rgb(255, 255, 255);
        let acento = rgb(p.color.0, p.color.1, p.color.2);
        let tiempo_texto = INICIO.lock().unwrap().map_or(0, |i| i.elapsed().as_millis() as i64);
        if !expandida {
            let m = esc(14);
            let art = esc(24);
            let y = (r.bottom - art) / 2;
            portada(mem, p, RECT { left: m, top: y, right: m + art, bottom: y + art }, esc(6));
            let anchura_ondas = 5 * esc(4) + 4 * esc(3);
            ondas(mem, r.right - m, r.bottom / 2, p.reproduciendo, t, acento);
            let (izq, der) = (m + art + esc(10), r.right - m - anchura_ondas - esc(10));
            // título (blanco) · artista (gris): tres tramos dibujados uno tras otro
            let (w_titulo, w_punto, w_artista) = (medir(mem, &p.titulo, 14, 600), medir(mem, "·", 14, 700), medir(mem, &p.artista, 14, 400));
            let hueco = esc(8);
            let ancho_texto = w_titulo + hueco + w_punto + hueco + w_artista;
            let desborda = ancho_texto > der - izq;
            DESBORDA.store(desborda, Ordering::Relaxed);
            let off = if desborda { desplazamiento(ancho_texto - (der - izq), tiempo_texto) } else { 0 };
            let _ = IntersectClipRect(mem, izq, 0, der, r.bottom);
            let tramo = |x: i32, w: i32| RECT { left: x, top: 0, right: x + w + 4, bottom: r.bottom };
            let x0 = izq - off;
            texto(mem, &p.titulo, tramo(x0, w_titulo), 14, 600, blanco, DRAW_TEXT_FORMAT(0));
            texto(mem, "·", tramo(x0 + w_titulo + hueco, w_punto), 14, 700, acento, DRAW_TEXT_FORMAT(0));
            texto(mem, &p.artista, tramo(x0 + w_titulo + 2 * hueco + w_punto, w_artista), 14, 400, rgb(170, 170, 170), DRAW_TEXT_FORMAT(0));
            SelectClipRgn(mem, None);
        } else {
            DESBORDA.store(false, Ordering::Relaxed);
            let m = esc(16);
            let art = esc(44);
            portada(mem, p, RECT { left: m, top: m, right: m + art, bottom: m + art }, esc(10));
            ondas(mem, r.right - m, m + art / 2, p.reproduciendo, t, acento);
            let izq = m + art + esc(10);
            let der = r.right - m - 5 * esc(4) - 4 * esc(3) - esc(10);
            let elipsis = DT_END_ELLIPSIS;
            texto(mem, &p.titulo, RECT { left: izq, top: m, right: der, bottom: m + art / 2 + esc(2) }, 16, 700, blanco, elipsis);
            texto(mem, &p.artista, RECT { left: izq, top: m + art / 2, right: der, bottom: m + art }, 13, 400, rgb(170, 170, 170), elipsis);

            // barra de progreso: posición extrapolada mientras suena
            let pos = if p.reproduciendo { p.pos_ms + p.desde.elapsed().as_millis() as i64 } else { p.pos_ms };
            let frac = if p.dur_ms > 0 { (pos as f64 / p.dur_ms as f64).clamp(0.0, 1.0) } else { 0.0 };
            let (y, h, largo) = (esc(80), esc(4), r.right - 2 * m);
            rect_lleno(mem, RECT { left: m, top: y, right: m + largo, bottom: y + h }, rgb(70, 70, 70), esc(2));
            if frac > 0.0 {
                rect_lleno(mem, RECT { left: m, top: y, right: m + (largo as f64 * frac) as i32, bottom: y + h }, blanco, esc(2));
            }

            let b = botones(r.right);
            let (cy, s) = (b[1].top + (b[1].bottom - b[1].top) / 2, esc(8));
            icono(mem, Icono::Anterior, b[0].left + (b[0].right - b[0].left) / 2, cy, s, blanco);
            icono(mem, Icono::Siguiente, b[2].left + (b[2].right - b[2].left) / 2, cy, s, blanco);
            // play/pausa: círculo blanco con icono negro
            let cb = rect_lleno_circulo(mem, b[1]);
            icono(mem, if p.reproduciendo { Icono::Pausa } else { Icono::Play }, cb.0, cb.1, esc(7), rgb(0, 0, 0));
        }
    }
}

/// Círculo blanco del botón central; devuelve su centro.
unsafe fn rect_lleno_circulo(hdc: HDC, r: RECT) -> (i32, i32) {
    unsafe {
        rect_lleno(hdc, r, rgb(255, 255, 255), (r.right - r.left) / 2);
        ((r.left + r.right) / 2, (r.top + r.bottom) / 2)
    }
}

// ---------- ventana ----------

fn dimensiones(expandida: bool) -> (i32, i32) {
    let (a, h) = if expandida { EXPANDIDO } else { COMPACTO };
    (esc(a), esc(h))
}

/// Ajusta tamaño y forma de la ventana; sigue centrada y pegada arriba.
unsafe fn aplicar_tamano(hwnd: HWND) {
    unsafe {
        let (an, al) = dimensiones(EXPANDIDA.load(Ordering::Relaxed));
        let x = (GetSystemMetrics(SM_CXSCREEN) - an) / 2;
        let _ = SetWindowPos(hwnd, None, x, 0, an, al, SWP_NOACTIVATE | SWP_NOZORDER);
        // esquinas superiores rectas: el redondeo sube por encima del borde
        let r = esc(RADIO);
        SetWindowRgn(hwnd, Some(CreateRoundRectRgn(0, -r, an + 1, al + 1, 2 * r, 2 * r)), true);
    }
}

/// El temporizador solo corre mientras suena algo o el título se desplaza.
unsafe fn ajustar_timer(hwnd: HWND) {
    unsafe {
        let animar = PISTA.lock().unwrap().as_ref().is_some_and(|p| p.reproduciendo) || DESBORDA.load(Ordering::Relaxed);
        if TIMER.swap(animar, Ordering::Relaxed) != animar {
            if animar {
                SetTimer(Some(hwnd), 1, 50, None);
            } else {
                let _ = KillTimer(Some(hwnd), 1);
            }
        }
    }
}

/// Gancho de ratón global: avisa de los clics fuera de la isla para compactarla. La ventana no tiene el
/// foco (no roba el de otras apps), así que no recibiría esos clics; el gancho solo existe mientras
/// está expandida y no consume ningún clic.
unsafe extern "system" fn gancho_raton(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 && matches!(wp.0 as u32, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN) {
            let pt = (*(lp.0 as *const MSLLHOOKSTRUCT)).pt;
            let hwnd = HWND(VENTANA.load(Ordering::Relaxed) as *mut c_void);
            let mut r = RECT::default();
            if GetWindowRect(hwnd, &mut r).is_ok() && !PtInRect(&r, pt).as_bool() {
                let _ = PostMessageW(Some(hwnd), WM_COLAPSAR, WPARAM(0), LPARAM(0));
            }
        }
        CallNextHookEx(None, code, wp, lp)
    }
}

unsafe fn fijar_gancho(activo: bool) {
    unsafe {
        let actual = GANCHO.load(Ordering::Relaxed);
        if activo && actual == 0 {
            let hmod = GetModuleHandleW(None).ok();
            if let Ok(h) = SetWindowsHookExW(WH_MOUSE_LL, Some(gancho_raton), hmod.map(Into::into), 0) {
                GANCHO.store(h.0 as isize, Ordering::Relaxed);
            }
        } else if !activo && actual != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(actual as *mut c_void));
            GANCHO.store(0, Ordering::Relaxed);
        }
    }
}

unsafe fn cambiar_expansion(hwnd: HWND, expandir: bool) {
    unsafe {
        EXPANDIDA.store(expandir, Ordering::Relaxed);
        fijar_gancho(expandir);
        aplicar_tamano(hwnd);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

unsafe extern "system" fn proc_ventana(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_APP => {
                let hay = PISTA.lock().unwrap().is_some();
                if !hay {
                    EXPANDIDA.store(false, Ordering::Relaxed);
                    fijar_gancho(false);
                    aplicar_tamano(hwnd);
                }
                let _ = ShowWindow(hwnd, if hay { SW_SHOWNOACTIVATE } else { SW_HIDE });
                if hay {
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                ajustar_timer(hwnd);
                LRESULT(0)
            }
            WM_COLAPSAR => {
                if EXPANDIDA.load(Ordering::Relaxed) {
                    cambiar_expansion(hwnd, false);
                }
                LRESULT(0)
            }
            WM_TIMER => {
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize), // clic sin robar el foco
            WM_SETCURSOR => {
                // manita donde un clic hace algo: la isla compacta (expande) y los botones del Expanded
                let mut p = POINT::default();
                let mut r = RECT::default();
                let _ = GetCursorPos(&mut p);
                let _ = ScreenToClient(hwnd, &mut p);
                let _ = GetClientRect(hwnd, &mut r);
                let manita = !EXPANDIDA.load(Ordering::Relaxed) || botones(r.right).iter().any(|b| PtInRect(b, p).as_bool());
                SetCursor(LoadCursorW(None, if manita { IDC_HAND } else { IDC_ARROW }).ok());
                LRESULT(1)
            }
            WM_LBUTTONDOWN => {
                let (x, y) = ((lp.0 & 0xFFFF) as i16 as i32, ((lp.0 >> 16) & 0xFFFF) as i16 as i32);
                if !EXPANDIDA.load(Ordering::Relaxed) {
                    cambiar_expansion(hwnd, true);
                } else {
                    let mut r = RECT::default();
                    let _ = GetClientRect(hwnd, &mut r);
                    let acciones = [Accion::Anterior, Accion::PlayPausa, Accion::Siguiente];
                    match botones(r.right).iter().position(|b| PtInRect(b, punto(x, y)).as_bool()) {
                        Some(i) => media::accion(acciones.into_iter().nth(i).unwrap()),
                        None => cambiar_expansion(hwnd, false),
                    }
                }
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let mut r = RECT::default();
                let _ = GetClientRect(hwnd, &mut r);
                // doble búfer: sin parpadeo
                let mem = CreateCompatibleDC(Some(hdc));
                let bmp = CreateCompatibleBitmap(hdc, r.right, r.bottom);
                let bmp_viejo = SelectObject(mem, bmp.into());
                FillRect(mem, &r, HBRUSH(GetStockObject(BLACK_BRUSH).0));
                SetBkMode(mem, TRANSPARENT);
                let pista = PISTA.lock().unwrap().clone();
                if let Some(p) = pista {
                    pintar(mem, r, &p, EXPANDIDA.load(Ordering::Relaxed));
                }
                let _ = BitBlt(hdc, 0, 0, r.right, r.bottom, Some(mem), 0, 0, SRCCOPY);
                SelectObject(mem, bmp_viejo);
                let _ = DeleteObject(bmp.into());
                let _ = DeleteDC(mem);
                let _ = EndPaint(hwnd, &ps);
                ajustar_timer(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                fijar_gancho(false);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

fn main() -> windows::core::Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        RoInitialize(RO_INIT_MULTITHREADED)?;
        *DPI.lock().unwrap() = GetDpiForSystem() as i32;

        let hinst = GetModuleHandleW(None)?;
        let clase = w!("IslaWin");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc_ventana),
            hCursor: LoadCursorW(None, IDC_ARROW)?, // sin cursor de clase Windows deja el de "ocupado"
            hInstance: hinst.into(),
            lpszClassName: clase,
            ..Default::default()
        });
        let (an, al) = dimensiones(false);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            clase, PCWSTR::null(), WS_POPUP, 0, 0, an, al, None, None, Some(hinst.into()), None,
        )?;
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
        VENTANA.store(hwnd.0 as isize, Ordering::Relaxed);
        aplicar_tamano(hwnd);
        media::iniciar(hwnd.0 as isize)?;

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}
