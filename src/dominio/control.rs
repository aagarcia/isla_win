//! Órdenes que la interfaz puede dar al reproductor, sin saber quién lo implementa.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Accion {
    Anterior,
    PlayPausa,
    Siguiente,
}

/// Quien sabe ejecutar acciones sobre la reproducción actual (hoy SMTC; mañana otro origen).
pub trait ControlMedios: Send + Sync {
    fn ejecutar(&self, accion: Accion);
}
