//! Iconos de control dibujados con formas simples (a falta de Segoe Fluent Icons).

use super::gdi::Lienzo;
use crate::dominio::color::Rgb;
use windows::Win32::Foundation::{POINT, RECT};

#[derive(Clone, Copy, PartialEq)]
pub enum Icono {
    Anterior,
    Siguiente,
    Play,
    Pausa,
}

/// Dibuja `icono` centrado en (`cx`, `cy`); `medio` es la mitad de su lado.
pub fn dibujar(lienzo: &Lienzo, icono: Icono, cx: i32, cy: i32, medio: i32, color: Rgb) {
    let s = medio;
    let barra = |izquierda, derecha| RECT { left: izquierda, top: cy - s, right: derecha, bottom: cy + s };
    let punto = |x, y| POINT { x, y };
    match icono {
        Icono::Anterior => {
            lienzo.rect_lleno(barra(cx - s, cx - s + s / 3), color, 0);
            lienzo.poligono_suave(&[punto(cx + s, cy - s), punto(cx + s, cy + s), punto(cx - s + s / 2, cy)], color);
        }
        Icono::Siguiente => {
            lienzo.rect_lleno(barra(cx + s - s / 3, cx + s), color, 0);
            lienzo.poligono_suave(&[punto(cx - s, cy - s), punto(cx - s, cy + s), punto(cx + s - s / 2, cy)], color);
        }
        Icono::Play => {
            lienzo.poligono_suave(&[punto(cx - s * 2 / 3, cy - s), punto(cx - s * 2 / 3, cy + s), punto(cx + s, cy)], color);
        }
        Icono::Pausa => {
            let (ancho, hueco) = (s * 2 / 3, s / 2);
            lienzo.rect_lleno(barra(cx - hueco / 2 - ancho, cx - hueco / 2), color, 0);
            lienzo.rect_lleno(barra(cx + hueco / 2, cx + hueco / 2 + ancho), color, 0);
        }
    }
}
