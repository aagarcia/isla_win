//! Medidas y colores del diseño (ver `mockup_ds`). Medidas en px a 96 dpi.

use super::gdi::Fuente;
use crate::dominio::color::Rgb;

pub const MINIMO: (i32, i32) = (100, 30);
pub const COMPACTO: (i32, i32) = (270, 42);
pub const EXPANDIDO: (i32, i32) = (420, 150);
/// Radio de las esquinas inferiores de la isla.
pub const RADIO_ISLA: i32 = 22;

pub const FUENTE: &str = "Segoe UI Variable Text";
pub const TITULO_COMPACTO: Fuente = Fuente { px: 14, peso: 600 };
pub const ARTISTA_COMPACTO: Fuente = Fuente { px: 14, peso: 400 };
pub const SEPARADOR_COMPACTO: Fuente = Fuente { px: 14, peso: 700 };
pub const TITULO_EXPANDIDO: Fuente = Fuente { px: 16, peso: 700 };
pub const ARTISTA_EXPANDIDO: Fuente = Fuente { px: 13, peso: 400 };

pub const NEGRO: Rgb = Rgb(0, 0, 0);
pub const BLANCO: Rgb = Rgb(255, 255, 255);
pub const GRIS_TEXTO: Rgb = Rgb(170, 170, 170);
pub const GRIS_BARRA: Rgb = Rgb(70, 70, 70);
pub const GRIS_SIN_PORTADA: Rgb = Rgb(48, 48, 56);
pub const GRIS_NOTA: Rgb = Rgb(150, 150, 160);

/// Intervalo de repintado mientras algo se anima (ms): ~15 fotogramas por segundo bastan para la onda.
pub const INTERVALO_ANIMACION_MS: u32 = 66;
/// Velocidad del desplazamiento del título largo (px/s a 96 dpi).
pub const VELOCIDAD_DESPLAZAMIENTO: i32 = 30;
