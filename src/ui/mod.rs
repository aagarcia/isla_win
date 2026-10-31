//! Interfaz: ventana, dibujo y vistas. Solo conoce el dominio y `estado`, nunca a SMTC.

mod bandeja;
mod escala;
mod gancho;
mod gdi;
mod iconos;
mod tema;
mod textos;
mod tooltip;
mod vista;
mod vista_musica;
pub mod ventana;

pub use escala::Escala;
