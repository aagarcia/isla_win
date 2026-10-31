//! Globos de ayuda de los botones del estado expandido. Un control de Windows con una zona por botón;
//! el texto se pide al pasar el ratón (`TTN_GETDISPINFOW`), así sigue al idioma y al play/pausa.

use std::sync::atomic::{AtomicIsize, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PWSTR;

/// Tooltip compartido (un solo hilo de interfaz).
static TOOLTIP: AtomicIsize = AtomicIsize::new(0);
const MAX_BOTONES: usize = 3;
/// Espera antes de mostrar el globo, en ms.
const RETARDO_MS: isize = 400;

pub fn crear(ventana: HWND) {
    unsafe {
        let _ = InitCommonControlsEx(&INITCOMMONCONTROLSEX { dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32, dwICC: ICC_WIN95_CLASSES });
        let Ok(tooltip) = CreateWindowExW(
            WS_EX_TOPMOST, TOOLTIPS_CLASSW, None, WS_POPUP | WINDOW_STYLE(TTS_ALWAYSTIP | TTS_NOPREFIX),
            CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, Some(ventana), None, None, None,
        ) else { return };
        SendMessageW(tooltip, TTM_SETDELAYTIME, Some(WPARAM(TTDT_INITIAL as usize)), Some(LPARAM(RETARDO_MS)));
        TOOLTIP.store(tooltip.0 as isize, Ordering::Relaxed);
    }
}

fn tooltip() -> Option<HWND> {
    match TOOLTIP.load(Ordering::Relaxed) {
        0 => None,
        valor => Some(HWND(valor as *mut _)),
    }
}

/// `TTTOOLINFOW` sin los campos de la versión 6 de comctl32: sin manifiesto, Windows carga la 5 y rechaza el tamaño completo.
fn herramienta(ventana: HWND, indice: usize, zona: RECT) -> TTTOOLINFOW {
    TTTOOLINFOW {
        cbSize: std::mem::offset_of!(TTTOOLINFOW, lParam) as u32,
        uFlags: TTF_SUBCLASS,
        hwnd: ventana,
        uId: indice,
        rect: zona,
        lpszText: PWSTR(-1isize as *mut u16), // LPSTR_TEXTCALLBACKW
        ..Default::default()
    }
}

/// Quita las zonas anteriores y pone una por botón (vacío = sin globos, p. ej. al compactar).
pub fn colocar(ventana: HWND, zonas: &[RECT]) {
    let Some(tooltip) = tooltip() else { return };
    unsafe {
        for indice in 0..MAX_BOTONES {
            let info = herramienta(ventana, indice, RECT::default());
            SendMessageW(tooltip, TTM_DELTOOLW, None, Some(LPARAM(&info as *const _ as isize)));
        }
        for (indice, &zona) in zonas.iter().take(MAX_BOTONES).enumerate() {
            let info = herramienta(ventana, indice, zona);
            SendMessageW(tooltip, TTM_ADDTOOLW, None, Some(LPARAM(&info as *const _ as isize)));
        }
    }
}

/// Si `lparam` es la petición de texto de un globo, el índice del botón.
pub unsafe fn indice_pedido(lparam: LPARAM) -> Option<usize> {
    let cabecera = unsafe { &*(lparam.0 as *const NMHDR) };
    (cabecera.code == TTN_GETDISPINFOW).then_some(cabecera.idFrom)
}

/// Escribe el texto del globo en la petición.
pub unsafe fn responder(lparam: LPARAM, texto: &str) {
    let info = unsafe { &mut *(lparam.0 as *mut NMTTDISPINFOW) };
    let n = texto.encode_utf16().count().min(info.szText.len() - 1);
    for (destino, c) in info.szText.iter_mut().zip(texto.encode_utf16().take(n)) {
        *destino = c;
    }
    info.szText[n] = 0;
    info.lpszText = PWSTR(info.szText.as_mut_ptr());
}
