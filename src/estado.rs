//! Estado compartido entre el origen de datos (SMTC, hilos de Windows) y la interfaz (hilo de la ventana).
//! Es el único punto de contacto: ninguno de los dos lados conoce al otro.

use crate::dominio::control::{Accion, ControlMedios};
use crate::dominio::pista::Pista;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

static PISTA: Mutex<Option<Arc<Pista>>> = Mutex::new(None);
static ULTIMO_CAMBIO: Mutex<Option<Instant>> = Mutex::new(None);
static PAUSADA_DESDE: Mutex<Option<Instant>> = Mutex::new(None);
static CONTROL: OnceLock<Box<dyn ControlMedios>> = OnceLock::new();

/// La pista actual. Es un `Arc`: leerla no copia los textos ni la portada.
pub fn pista() -> Option<Arc<Pista>> {
    PISTA.lock().unwrap().clone()
}

/// Guarda la pista actual (None = nada suena). Devuelve true si cambió la canción o apareció/desapareció.
pub fn publicar(nueva: Option<Pista>) -> bool {
    anotar_pausa(nueva.as_ref());
    let mut actual = PISTA.lock().unwrap();
    let cambio = match (&*actual, &nueva) {
        (Some(a), Some(b)) => !a.es_la_misma_cancion(b),
        (None, None) => false,
        _ => true,
    };
    if cambio {
        *ULTIMO_CAMBIO.lock().unwrap() = Some(Instant::now());
    }
    *actual = nueva.map(Arc::new);
    cambio
}

/// Recuerda desde cuándo está en pausa (el primer instante, no cada evento) y lo olvida al sonar o sin sesión.
fn anotar_pausa(nueva: Option<&Pista>) {
    let mut desde = PAUSADA_DESDE.lock().unwrap();
    match nueva {
        Some(p) if !p.reproduciendo => {
            desde.get_or_insert_with(Instant::now);
        }
        _ => *desde = None,
    }
}

/// Desde cuándo está en pausa la música; None si suena o no hay sesión.
pub fn pausada_desde() -> Option<Instant> {
    *PAUSADA_DESDE.lock().unwrap()
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
