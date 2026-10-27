//! Contrato de una vista de la isla. La ventana solo conoce este trait: música hoy; batería, descarga
//! o llamada mañana, cada una en su archivo, sin tocar la ventana.

use super::escala::Escala;
use super::gdi::Lienzo;
use super::vista_musica::VistaMusica;
use windows::Win32::Foundation::{POINT, RECT};

pub trait Vista {
    /// Pinta el estado minimal: solo lo esencial (en música, la onda).
    fn minimal(&self, lienzo: &Lienzo, area: RECT);

    /// Pinta el estado compacto. Devuelve true si el contenido no cabe y se desplaza (necesita repintado).
    fn compacta(&self, lienzo: &Lienzo, area: RECT) -> bool;

    fn expandida(&self, lienzo: &Lienzo, area: RECT);

    /// ¿Hay un botón bajo `punto` en el estado expandido?
    fn hay_boton_en(&self, punto: POINT, area: RECT, escala: Escala) -> bool;

    /// Atiende un clic en el estado expandido. Devuelve true si pulsó un botón.
    fn pulsar(&self, punto: POINT, area: RECT, escala: Escala) -> bool;

    /// ¿Tiene algo en movimiento (onda, progreso) que exija repintar?
    fn animando(&self) -> bool;
}

/// La vista que corresponde mostrar ahora, o None si no hay nada que mostrar.
pub fn activa() -> Option<Box<dyn Vista>> {
    VistaMusica::actual().map(|v| Box::new(v) as Box<dyn Vista>)
}
