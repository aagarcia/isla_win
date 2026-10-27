//! Portada de la canción actual.
//!
//! SMTC publica la imagen con retraso y, al principio, puede devolver la de la canción anterior:
//! no se da por buena hasta que llega y difiere de la anterior (o se agotan los intentos: mismo álbum).

use crate::dominio::pista::Portada;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex};
use windows::Graphics::Imaging::{
    BitmapAlphaMode, BitmapDecoder, BitmapInterpolationMode, BitmapPixelFormat, BitmapTransform,
    ColorManagementMode, ExifOrientationMode,
};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties as Propiedades;

/// Lado de la portada reducida: ~36 KB en memoria.
const LADO: u32 = 96;
/// Intentos tras los que una portada igual a la anterior se acepta (canciones del mismo álbum).
const INTENTOS_ANTES_DE_ACEPTAR_REPETIDA: u8 = 4;

struct Cache {
    clave: String,
    portada: Option<Arc<Portada>>,
    huella: u64,
    huella_anterior: u64,
    intentos: u8,
}

static CACHE: Mutex<Cache> =
    Mutex::new(Cache { clave: String::new(), portada: None, huella: 0, huella_anterior: 0, intentos: 0 });

/// Portada de la canción identificada por `clave`; None mientras Windows aún no la entregue.
pub fn de(propiedades: &Propiedades, clave: &str) -> Option<Arc<Portada>> {
    let huella_anterior = {
        let mut cache = CACHE.lock().unwrap();
        if cache.clave != clave {
            let huella_anterior = cache.huella;
            *cache = Cache { clave: clave.to_string(), portada: None, huella: 0, huella_anterior, intentos: 0 };
        }
        if let Some(portada) = &cache.portada {
            return Some(portada.clone());
        }
        cache.huella_anterior
    };

    let leida = decodificar(propiedades); // sin el candado: decodificar es lento
    let mut cache = CACHE.lock().unwrap();
    if cache.clave != clave {
        return None; // cambió de canción mientras se leía
    }
    cache.intentos = cache.intentos.saturating_add(1);
    let portada = leida?;
    let huella = huella(&portada.bgra);
    let es_la_anterior = huella == huella_anterior && cache.intentos <= INTENTOS_ANTES_DE_ACEPTAR_REPETIDA;
    if es_la_anterior {
        return None;
    }
    let portada = Arc::new(portada);
    (cache.huella, cache.portada) = (huella, Some(portada.clone()));
    Some(portada)
}

fn huella(bgra: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bgra.hash(&mut hasher);
    hasher.finish()
}

fn decodificar(propiedades: &Propiedades) -> Option<Portada> {
    let flujo = propiedades.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
    let decodificador = BitmapDecoder::CreateAsync(&flujo).ok()?.join().ok()?;
    let transformacion = BitmapTransform::new().ok()?;
    transformacion.SetScaledWidth(LADO).ok()?;
    transformacion.SetScaledHeight(LADO).ok()?;
    transformacion.SetInterpolationMode(BitmapInterpolationMode::Fant).ok()?;
    let pixeles = decodificador
        .GetPixelDataTransformedAsync(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
            &transformacion,
            ExifOrientationMode::IgnoreExifOrientation,
            ColorManagementMode::DoNotColorManage,
        )
        .ok()?
        .join()
        .ok()?;
    Some(Portada { lado: LADO as i32, bgra: pixeles.DetachPixelData().ok()?.to_vec() })
}
