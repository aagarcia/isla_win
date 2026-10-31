//! Textos de la interfaz en español e inglés. El idioma sale del de Windows y se fija al primer uso.

use std::sync::atomic::{AtomicU8, Ordering};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

/// Idioma de la interfaz.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Idioma {
    Es,
    En,
}

pub struct Textos {
    pub mostrar: &'static str,
    pub ocultar: &'static str,
    pub reposo: &'static str,
    pub idioma: &'static str,
    pub cerrar: &'static str,
}

const ES: Textos = Textos { mostrar: "Mostrar isla", ocultar: "Ocultar isla", reposo: "Tamaño en reposo", idioma: "Idioma", cerrar: "Cerrar" };
const EN: Textos = Textos { mostrar: "Show island", ocultar: "Hide island", reposo: "Resting size", idioma: "Language", cerrar: "Close" };

/// Idioma primario de Windows (`LANGID & 0x3FF`): 0x0A es español; cualquier otro, inglés.
fn de_windows(langid: u16) -> Idioma {
    if langid & 0x3FF == 0x0A { Idioma::Es } else { Idioma::En }
}

/// Elección del usuario: 0 = aún no (se usa el idioma de Windows), 1 = español, 2 = inglés.
static ELEGIDO: AtomicU8 = AtomicU8::new(0);

pub fn fijar(idioma: Idioma) {
    ELEGIDO.store(if idioma == Idioma::Es { 1 } else { 2 }, Ordering::Relaxed);
}

pub fn idioma() -> Idioma {
    match ELEGIDO.load(Ordering::Relaxed) {
        1 => Idioma::Es,
        2 => Idioma::En,
        _ => de_windows(unsafe { GetUserDefaultUILanguage() }),
    }
}

pub fn actuales() -> &'static Textos {
    if idioma() == Idioma::Es { &ES } else { &EN }
}

/// Cadena UTF-16 terminada en nul, como la piden los menús de Win32.
pub fn ancho(texto: &str) -> Vec<u16> {
    texto.encode_utf16().chain(Some(0)).collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn cualquier_variante_del_espanol_usa_espanol() {
        for langid in [0x0C0A, 0x040A, 0x080A, 0x2C0A] {
            assert_eq!(de_windows(langid), Idioma::Es);
        }
    }

    #[test]
    fn el_resto_de_idiomas_usa_ingles() {
        for langid in [0x0409, 0x0809, 0x040C, 0x0000] {
            assert_eq!(de_windows(langid), Idioma::En);
        }
    }

    #[test]
    fn fijar_cambia_los_textos() {
        fijar(Idioma::En);
        assert_eq!(actuales().cerrar, "Close");
        fijar(Idioma::Es);
        assert_eq!(actuales().cerrar, "Cerrar");
    }

    #[test]
    fn ninguna_tabla_tiene_textos_vacios() {
        for t in [&ES, &EN] {
            assert!([t.mostrar, t.ocultar, t.reposo, t.idioma, t.cerrar].iter().all(|s| !s.is_empty()));
        }
    }

    #[test]
    fn ancho_termina_en_nul() {
        assert_eq!(ancho("ab"), [97, 98, 0]);
    }
}
