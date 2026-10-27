//! Origen de datos: la sesión multimedia de Windows (SMTC), vigilada por eventos.
//!
//! No conoce la interfaz: cada vez que algo cambia publica en `estado` y llama al callback `al_cambiar`.

mod lectura;
mod portada;

use crate::dominio::control::{Accion, ControlMedios};
use crate::estado;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as Gestor;

type AlCambiar = Arc<dyn Fn() + Send + Sync>;

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

/// shortcut: las suscripciones se acumulan cada vez que cambia la sesión actual; des-suscribir si se adopta.
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
    let _ = sesion.MediaPropertiesChanged(&al_evento!());
    let _ = sesion.PlaybackInfoChanged(&al_evento!());
    let _ = sesion.TimelinePropertiesChanged(&al_evento!());
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
    std::thread::spawn(move || {
        for espera in ESPERAS_PORTADA_MS {
            std::thread::sleep(Duration::from_millis(espera));
            if estado::pista().is_some_and(|p| p.portada.is_some()) {
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
