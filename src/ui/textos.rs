//! Textos de la interfaz en español e inglés. El idioma sale del de Windows y se fija al primer uso.

use std::sync::OnceLock;
use windows::Win32::Globalization::GetUserDefaultUILanguage;

pub struct Textos {
    pub mostrar: &'static str,
    pub ocultar: &'static str,
    pub reposo: &'static str,
    pub cerrar: &'static str,
}

const ES: Textos = Textos { mostrar: "Mostrar isla", ocultar: "Ocultar isla", reposo: "Tamaño en reposo", cerrar: "Cerrar" };
const EN: Textos = Textos { mostrar: "Show island", ocultar: "Hide island", reposo: "Resting size", cerrar: "Close" };

/// Idioma primario de Windows (`LANGID & 0x3FF`): 0x0A es español; cualquier otro, inglés.
fn para(langid: u16) -> &'static Textos {
    if langid & 0x3FF == 0x0A { &ES } else { &EN }
}

pub fn actuales() -> &'static Textos {
    static TEXTOS: OnceLock<&'static Textos> = OnceLock::new();
    TEXTOS.get_or_init(|| para(unsafe { GetUserDefaultUILanguage() }))
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
            assert_eq!(para(langid).cerrar, "Cerrar");
        }
    }

    #[test]
    fn el_resto_de_idiomas_usa_ingles() {
        for langid in [0x0409, 0x0809, 0x040C, 0x0000] {
            assert_eq!(para(langid).cerrar, "Close");
        }
    }

    #[test]
    fn ninguna_tabla_tiene_textos_vacios() {
        for t in [&ES, &EN] {
            assert!([t.mostrar, t.ocultar, t.reposo, t.cerrar].iter().all(|s| !s.is_empty()));
        }
    }

    #[test]
    fn ancho_termina_en_nul() {
        assert_eq!(ancho("ab"), [97, 98, 0]);
    }
}
