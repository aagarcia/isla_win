//! Lo que suena ahora: datos de la canción y su progreso.

use super::color::Rgb;
use std::sync::Arc;
use std::time::Instant;

/// Imagen BGRA ya reducida, lista para dibujar.
pub struct Portada {
    pub lado: i32,
    pub bgra: Vec<u8>,
}

#[derive(Clone)]
pub struct Pista {
    pub titulo: String,
    pub artista: String,
    pub reproduciendo: bool,
    /// Posición en el instante `medida_en`; mientras suena se extrapola con el reloj.
    pub pos_ms: i64,
    pub dur_ms: i64,
    pub medida_en: Instant,
    pub portada: Option<Arc<Portada>>,
    pub acento: Rgb,
}

impl Pista {
    pub fn posicion_ms(&self) -> i64 {
        if self.reproduciendo {
            self.pos_ms + self.medida_en.elapsed().as_millis() as i64
        } else {
            self.pos_ms
        }
    }

    /// Avance de 0 a 1; 0 si se desconoce la duración.
    pub fn progreso(&self) -> f64 {
        if self.dur_ms <= 0 {
            return 0.0;
        }
        (self.posicion_ms() as f64 / self.dur_ms as f64).clamp(0.0, 1.0)
    }

    pub fn es_la_misma_cancion(&self, otra: &Pista) -> bool {
        self.titulo == otra.titulo && self.artista == otra.artista
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::dominio::color::ROSA;

    fn pista(reproduciendo: bool, pos_ms: i64, dur_ms: i64) -> Pista {
        Pista {
            titulo: "Ayer".into(),
            artista: "Luis Miguel".into(),
            reproduciendo,
            pos_ms,
            dur_ms,
            medida_en: Instant::now(),
            portada: None,
            acento: ROSA,
        }
    }

    #[test]
    fn el_progreso_en_pausa_es_la_posicion_sobre_la_duracion() {
        assert_eq!(pista(false, 50_000, 100_000).progreso(), 0.5);
    }

    #[test]
    fn sin_duracion_el_progreso_es_cero() {
        assert_eq!(pista(true, 50_000, 0).progreso(), 0.0);
    }

    #[test]
    fn el_progreso_no_pasa_de_uno() {
        assert_eq!(pista(false, 200_000, 100_000).progreso(), 1.0);
    }

    #[test]
    fn misma_cancion_solo_si_coinciden_titulo_y_artista() {
        let a = pista(true, 0, 1);
        let mut b = pista(false, 9, 1);
        assert!(a.es_la_misma_cancion(&b));
        b.titulo = "Otra".into();
        assert!(!a.es_la_misma_cancion(&b));
    }
}
