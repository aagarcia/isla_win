//! Origen de datos: la sesión multimedia de Windows (SMTC), vigilada por eventos.
//!
//! No conoce la interfaz: cada vez que algo cambia publica en `estado` y llama al callback `al_cambiar`.

mod lectura;
mod portada;

use crate::dominio::control::{Accion, ControlMedios};
use crate::estado;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Sesion,
    GlobalSystemMediaTransportControlsSessionManager as Gestor,
};

type AlCambiar = Arc<dyn Fn() + Send + Sync>;

/// Número del último reintento de portada lanzado: los hilos anteriores ven que ya no son los vigentes y terminan.
static REINTENTO_VIGENTE: AtomicU64 = AtomicU64::new(0);

/// Manejadores de eventos de la sesión actual. Al sustituirla se des-suscriben los anteriores,
/// para que cada evento provoque una sola lectura por mucho que cambie la sesión.
struct Suscripcion {
    sesion: Sesion,
    propiedades: i64,
    reproduccion: i64,
    linea_de_tiempo: i64,
}

impl Drop for Suscripcion {
    fn drop(&mut self) {
        let _ = self.sesion.RemoveMediaPropertiesChanged(self.propiedades);
        let _ = self.sesion.RemovePlaybackInfoChanged(self.reproduccion);
        let _ = self.sesion.RemoveTimelinePropertiesChanged(self.linea_de_tiempo);
    }
}

static SUSCRIPCION: Mutex<Option<Suscripcion>> = Mutex::new(None);

/// Esperas entre intentos de leer la portada tras un cambio de canción: Spotify a veces la publica
/// con mucho retraso (medido: más de 14 s). El hilo termina en cuanto la tiene.
const ESPERAS_PORTADA_MS: [u64; 10] = [300, 700, 1500, 3000, 6000, 10_000, 15_000, 20_000, 30_000, 30_000];

/// Conecta con Windows y empieza a vigilar la sesión. Llamar una vez.
pub fn iniciar(al_cambiar: impl Fn() + Send + Sync + 'static) -> windows::core::Result<()> {
    let al_cambiar: AlCambiar = Arc::new(al_cambiar);
    let gestor = Gestor::RequestAsync()?.join()?;
    vigilar(&gestor, &al_cambiar);

    let al_cambiar_sesion = al_cambiar.clone();
    gestor.CurrentSessionChanged(&TypedEventHandler::new(move |gestor: windows::core::Ref<Gestor>, _| {
        if let Some(gestor) = gestor.as_ref() {
            vigilar(gestor, &al_cambiar_sesion);
        }
        Ok(())
    }))?;
    estado::registrar_control(Box::new(Smtc(Mutex::new(gestor))));
    Ok(())
}

fn vigilar(gestor: &Gestor, al_cambiar: &AlCambiar) {
    actualizar(gestor, al_cambiar);
    let Ok(sesion) = gestor.GetCurrentSession() else { return };

    // un manejador por tipo de evento (cada uno tiene sus propios tipos); todos releen la sesión
    macro_rules! al_evento {
        () => {{
            let (gestor, al_cambiar) = (gestor.clone(), al_cambiar.clone());
            TypedEventHandler::new(move |_, _| {
                actualizar(&gestor, &al_cambiar);
                Ok(())
            })
        }};
    }
    let suscripcion = (
        sesion.MediaPropertiesChanged(&al_evento!()),
        sesion.PlaybackInfoChanged(&al_evento!()),
        sesion.TimelinePropertiesChanged(&al_evento!()),
    );
    if let (Ok(propiedades), Ok(reproduccion), Ok(linea_de_tiempo)) = suscripcion {
        // al sustituir la anterior se descarta y se des-suscribe
        *SUSCRIPCION.lock().unwrap() = Some(Suscripcion { sesion, propiedades, reproduccion, linea_de_tiempo });
    }
}

fn actualizar(gestor: &Gestor, al_cambiar: &AlCambiar) {
    let nueva = lectura::leer(gestor);
    let suena_algo = nueva.is_some();
    if estado::publicar(nueva) && suena_algo {
        reintentar_portada(gestor.clone(), al_cambiar.clone());
    }
    al_cambiar();
}

fn reintentar_portada(gestor: Gestor, al_cambiar: AlCambiar) {
    let mio = REINTENTO_VIGENTE.fetch_add(1, Ordering::Relaxed) + 1;
    std::thread::spawn(move || {
        for espera in ESPERAS_PORTADA_MS {
            std::thread::sleep(Duration::from_millis(espera));
            let sustituido = REINTENTO_VIGENTE.load(Ordering::Relaxed) != mio; // cambió otra vez de canción
            if sustituido || estado::pista().is_some_and(|p| p.portada.is_some()) {
                break;
            }
            actualizar(&gestor, &al_cambiar);
        }
    });
}

/// Control de la reproducción a través de la sesión actual de SMTC.
struct Smtc(Mutex<Gestor>);

impl ControlMedios for Smtc {
    fn ejecutar(&self, accion: Accion) {
        let Ok(sesion) = self.0.lock().unwrap().GetCurrentSession() else { return };
        let _ = match accion {
            Accion::Anterior => sesion.TrySkipPreviousAsync(),
            Accion::PlayPausa => sesion.TryTogglePlayPauseAsync(),
            Accion::Siguiente => sesion.TrySkipNextAsync(),
        };
    }
}
