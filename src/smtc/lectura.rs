//! Traduce la sesión multimedia de Windows al modelo del dominio.

use super::portada;
use crate::dominio::color::{color_dominante, color_marca, ROSA};
use crate::dominio::pista::Pista;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use windows::Foundation::TimeSpan;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as Gestor,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Estado,
};

/// Los `DateTime` de WinRT cuentan en intervalos de 100 ns desde 1601; Unix desde 1970.
const TICKS_1601_A_1970: i64 = 116_444_736_000_000_000;
const TICKS_POR_MS: i64 = 10_000;

fn ms(t: TimeSpan) -> i64 {
    t.Duration / TICKS_POR_MS
}

/// Pista de la sesión actual si suena o está en pausa; None si no hay nada.
pub fn leer(gestor: &Gestor) -> Option<Pista> {
    let sesion = gestor.GetCurrentSession().ok()?;
    let estado = sesion.GetPlaybackInfo().ok()?.PlaybackStatus().ok()?;
    if estado != Estado::Playing && estado != Estado::Paused {
        return None;
    }
    let reproduciendo = estado == Estado::Playing;

    let propiedades = sesion.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let titulo = propiedades.Title().ok()?.to_string();
    let artista = propiedades.Artist().ok()?.to_string();

    let linea_de_tiempo = sesion.GetTimelineProperties().ok()?;
    let inicio = ms(linea_de_tiempo.StartTime().ok()?);
    let dur_ms = (ms(linea_de_tiempo.EndTime().ok()?) - inicio).max(0);
    let mut pos_ms = ms(linea_de_tiempo.Position().ok()?) - inicio;
    if reproduciendo {
        // la posición está medida en LastUpdatedTime: se suma lo transcurrido desde entonces
        let ahora = (SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_nanos() / 100) as i64 + TICKS_1601_A_1970;
        pos_ms += ((ahora - linea_de_tiempo.LastUpdatedTime().ok()?.UniversalTime) / TICKS_POR_MS).max(0);
    }

    let portada = portada::de(&propiedades, &format!("{titulo}\u{1}{artista}"));
    let aumid = sesion.SourceAppUserModelId().map(|id| id.to_string()).unwrap_or_default();
    let acento = color_marca(&aumid)
        .or_else(|| portada.as_ref().and_then(|p| color_dominante(&p.bgra)))
        .unwrap_or(ROSA);

    Some(Pista {
        titulo,
        artista,
        reproduciendo,
        pos_ms: pos_ms.clamp(0, dur_ms),
        dur_ms,
        medida_en: Instant::now(),
        portada,
        acento,
    })
}
