//! Lo que el usuario elige desde el icono de la bandeja.

use super::visibilidad::Visibilidad;

/// Tamaño de la isla en reposo. El expandido no se elige: se abre con un clic.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Reposo {
    Minimal,
    Compact,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Preferencias {
    pub reposo: Reposo,
    /// El usuario la ocultó a mano: no se ve aunque suene música, hasta que la muestre de nuevo.
    pub oculta_por_usuario: bool,
}

impl Default for Preferencias {
    fn default() -> Self {
        Self { reposo: Reposo::Compact, oculta_por_usuario: false }
    }
}

impl Preferencias {
    /// Combina la visibilidad automática (música, inactividad) con la decisión del usuario:
    /// ocultar a mano gana siempre; mostrar solo deja ver lo que la regla automática permite.
    pub fn aplicar(&self, automatica: Visibilidad) -> Visibilidad {
        if self.oculta_por_usuario { Visibilidad::Oculta } else { automatica }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::time::Duration;

    #[test]
    fn por_defecto_compact_y_visible() {
        assert_eq!(Preferencias::default(), Preferencias { reposo: Reposo::Compact, oculta_por_usuario: false });
    }

    #[test]
    fn oculta_a_mano_gana_aunque_suene() {
        let p = Preferencias { oculta_por_usuario: true, ..Default::default() };
        assert_eq!(p.aplicar(Visibilidad::Visible), Visibilidad::Oculta);
        assert_eq!(p.aplicar(Visibilidad::VisibleDurante(Duration::from_secs(5))), Visibilidad::Oculta);
    }

    #[test]
    fn mostrar_respeta_la_regla_automatica() {
        let p = Preferencias::default();
        assert_eq!(p.aplicar(Visibilidad::Visible), Visibilidad::Visible);
        assert_eq!(p.aplicar(Visibilidad::Oculta), Visibilidad::Oculta); // p. ej. 3 min de pausa
    }
}
