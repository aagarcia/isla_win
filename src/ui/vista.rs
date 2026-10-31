//! Contrato de una vista de la isla. La ventana solo conoce este trait: música hoy; batería, descarga
//! o llamada mañana, cada una en su archivo, sin tocar la ventana.
//!
//! Cada vista separa lo que **no cambia** entre fotogramas (portada, textos, botones: capa estática, que se
//! pinta una vez) de lo que **se mueve** (onda, progreso, texto desplazándose: se pinta en cada fotograma).

use super::escala::Escala;
use crate::dominio::control::Accion;
use super::gdi::Lienzo;
use super::vista_musica::VistaMusica;
use windows::Win32::Foundation::{POINT, RECT};

/// Los tres tamaños de la isla.
#[derive(Clone, Copy, PartialEq, Hash)]
pub enum Modo {
    Minimal,
    Compacto,
    Expandido,
}

pub trait Vista {
    /// Huella de lo estático: mientras no cambie, no se repinta la capa estática.
    fn huella_estatica(&self, modo: Modo, area: RECT) -> u64;

    /// Pinta lo que no cambia entre fotogramas.
    fn pintar_estatico(&self, lienzo: &Lienzo, area: RECT, modo: Modo);

    /// Pinta lo que se mueve. Devuelve true si el contenido sigue en movimiento (p. ej. un título desplazándose).
    fn pintar_dinamico(&self, lienzo: &Lienzo, area: RECT, modo: Modo) -> bool;

    /// ¿Hay un botón bajo `punto` en el estado expandido?
    fn hay_boton_en(&self, punto: POINT, area: RECT, escala: Escala) -> bool;

    /// Zona y acción de cada botón del estado expandido (para los globos de ayuda).
    fn zonas_botones(&self, area: RECT, escala: Escala) -> Vec<(RECT, Accion)>;

    /// Texto del globo de ayuda de un botón.
    fn texto_boton(&self, accion: Accion) -> &'static str;

    /// Atiende un clic en el estado expandido. Devuelve true si pulsó un botón.
    fn pulsar(&self, punto: POINT, area: RECT, escala: Escala) -> bool;

    /// ¿Tiene algo en movimiento permanente (onda) que exija repintar?
    fn animando(&self) -> bool;
}

/// La vista que corresponde mostrar ahora, o None si no hay nada que mostrar.
pub fn activa() -> Option<Box<dyn Vista>> {
    VistaMusica::actual().map(|v| Box::new(v) as Box<dyn Vista>)
}
