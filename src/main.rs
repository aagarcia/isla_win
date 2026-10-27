#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Isla estilo "Dynamic Island" para Windows, pegada al borde superior (portátiles con la cámara en el marco).
//!
//! Capas (cada una solo conoce a las de abajo):
//! - `ui`: ventana, dibujo y vistas.
//! - `estado`: lo que suena y el control de la reproducción; punto de contacto entre `ui` y `smtc`.
//! - `smtc`: origen de datos (sesión multimedia de Windows).
//! - `dominio`: modelo y reglas puras, sin Windows.

mod dominio;
mod estado;
mod smtc;
mod ui;

use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

fn main() -> windows::core::Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        RoInitialize(RO_INIT_MULTITHREADED)?;
        ui::Escala::fijar(GetDpiForSystem() as i32);
    }
    let ventana = ui::ventana::crear()?.0 as isize;
    smtc::iniciar(move || ui::ventana::avisar(ventana))?;
    ui::ventana::bucle_de_mensajes();
    Ok(())
}
