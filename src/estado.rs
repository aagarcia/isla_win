//! Estado compartido entre el origen de datos (SMTC, hilos de Windows) y la interfaz (hilo de la ventana).
//! Es el único punto de contacto: ninguno de los dos lados conoce al otro.

use crate::dominio::control::{Accion, ControlMedios};
use crate::dominio::pista::Pista;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static PISTA: Mutex<Option<Pista>> = Mutex::new(None);
static ULTIMO_CAMBIO: Mutex<Option<Instant>> = Mutex::new(None);
static CONTROL: OnceLock<Box<dyn ControlMedios>> = OnceLock::new();

pub fn pista() -> Option<Pista> {
    PISTA.lock().unwrap().clone()
}

/// Guarda la pista actual (None = nada suena). Devuelve true si cambió la canción o apareció/desapareció.
pub fn publicar(nueva: Option<Pista>) -> bool {
    let mut actual = PISTA.lock().unwrap();
    let cambio = match (&*actual, &nueva) {
        (Some(a), Some(b)) => !a.es_la_misma_cancion(b),
        (None, None) => false,
        _ => true,
    };
    if cambio {
        *ULTIMO_CAMBIO.lock().unwrap() = Some(Instant::now());
    }
    *actual = nueva;
    cambio
}

/// Milisegundos desde que cambió la canción: reinicia el desplazamiento del título.
pub fn ms_desde_el_cambio() -> i64 {
    ULTIMO_CAMBIO.lock().unwrap().map_or(0, |t| t.elapsed().as_millis() as i64)
}

pub fn registrar_control(control: Box<dyn ControlMedios>) {
    let _ = CONTROL.set(control);
}

pub fn enviar(accion: Accion) {
    if let Some(control) = CONTROL.get() {
        control.ejecutar(accion);
    }
}
