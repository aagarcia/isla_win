//! Escala de la pantalla: todas las medidas del diseño están en píxeles a 96 dpi.

use std::sync::atomic::{AtomicI32, Ordering};

const DPI_BASE: i32 = 96;
static DPI: AtomicI32 = AtomicI32::new(DPI_BASE);

#[derive(Clone, Copy)]
pub struct Escala(i32);

impl Escala {
    /// Fija los dpi del sistema (una vez, al arrancar).
    pub fn fijar(dpi: i32) {
        DPI.store(dpi, Ordering::Relaxed);
    }

    pub fn actual() -> Self {
        Self(DPI.load(Ordering::Relaxed))
    }

    /// Convierte una medida de diseño (96 dpi) a píxeles reales.
    pub fn px(self, medida: i32) -> i32 {
        medida * self.0 / DPI_BASE
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn al_125_por_ciento_270_pasa_a_337() {
        assert_eq!(Escala(120).px(270), 337);
    }

    #[test]
    fn a_96_dpi_no_cambia() {
        assert_eq!(Escala(96).px(42), 42);
    }
}
