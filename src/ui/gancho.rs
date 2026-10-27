//! Gancho de ratón global: detecta los clics fuera de la isla para compactarla.
//!
//! La ventana no tiene el foco (para no quitárselo a otras apps), así que no recibiría esos clics.
//! El gancho solo existe mientras la isla está expandida y no consume ningún clic.

use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::Graphics::Gdi::PtInRect;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Mensaje a la ventana: hubo un clic fuera de ella.
pub const WM_CLIC_FUERA: u32 = WM_APP + 1;

static GANCHO: AtomicIsize = AtomicIsize::new(0); // HHOOK instalado (0 = ninguno)
static VENTANA: AtomicIsize = AtomicIsize::new(0);

pub fn activar(ventana: HWND) {
    if GANCHO.load(Ordering::Relaxed) != 0 {
        return;
    }
    VENTANA.store(ventana.0 as isize, Ordering::Relaxed);
    unsafe {
        let modulo = GetModuleHandleW(None).ok();
        if let Ok(gancho) = SetWindowsHookExW(WH_MOUSE_LL, Some(al_pulsar_raton), modulo.map(Into::into), 0) {
            GANCHO.store(gancho.0 as isize, Ordering::Relaxed);
        }
    }
}

pub fn desactivar() {
    let gancho = GANCHO.swap(0, Ordering::Relaxed);
    if gancho != 0 {
        unsafe {
            let _ = UnhookWindowsHookEx(HHOOK(gancho as *mut c_void));
        }
    }
}

unsafe extern "system" fn al_pulsar_raton(codigo: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let es_clic = matches!(wparam.0 as u32, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN);
        if codigo >= 0 && es_clic {
            let punto = (*(lparam.0 as *const MSLLHOOKSTRUCT)).pt;
            let ventana = HWND(VENTANA.load(Ordering::Relaxed) as *mut c_void);
            let mut zona = RECT::default();
            if GetWindowRect(ventana, &mut zona).is_ok() && !PtInRect(&zona, punto).as_bool() {
                let _ = PostMessageW(Some(ventana), WM_CLIC_FUERA, WPARAM(0), LPARAM(0));
            }
        }
        CallNextHookEx(None, codigo, wparam, lparam)
    }
}
