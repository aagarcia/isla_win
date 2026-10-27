//! Cuándo se ve la isla: mientras suena, y un rato después de pausar; luego se oculta hasta que vuelva a sonar.

use std::time::{Duration, Instant};

/// Tiempo que la isla sigue visible tras pausar o parar la música.
pub const LIMITE_INACTIVIDAD: Duration = Duration::from_secs(3 * 60);

#[derive(Debug, PartialEq)]
pub enum Visibilidad {
    Visible,
    /// En pausa: sigue visible y hay que ocultarla cuando pase este tiempo.
    VisibleDurante(Duration),
    Oculta,
}

/// `reproduciendo`: None si no hay sesión; Some(true) suena; Some(false) está en pausa.
/// `pausada_desde`: desde cuándo está en pausa (None si suena o no hay sesión).
///
/// Es una función del estado y del reloj, no de los eventos: los eventos de una canción en pausa
/// pasada la espera no la vuelven a mostrar; solo lo hace volver a reproducir.
pub fn visibilidad(
    reproduciendo: Option<bool>,
    pausada_desde: Option<Instant>,
    ahora: Instant,
    limite: Duration,
) -> Visibilidad {
    match (reproduciendo, pausada_desde) {
        (None, _) => Visibilidad::Oculta,
        (Some(true), _) | (Some(false), None) => Visibilidad::Visible,
        (Some(false), Some(desde)) => {
            let en_pausa = ahora.saturating_duration_since(desde);
            match limite.checked_sub(en_pausa) {
                Some(resto) if !resto.is_zero() => Visibilidad::VisibleDurante(resto),
                _ => Visibilidad::Oculta,
            }
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const LIMITE: Duration = Duration::from_secs(180);

    fn hace(ahora: Instant, segundos: u64) -> Option<Instant> {
        Some(ahora - Duration::from_secs(segundos))
    }

    #[test]
    fn sin_sesion_esta_oculta() {
        assert_eq!(visibilidad(None, None, Instant::now(), LIMITE), Visibilidad::Oculta);
    }

    #[test]
    fn sonando_esta_visible() {
        assert_eq!(visibilidad(Some(true), None, Instant::now(), LIMITE), Visibilidad::Visible);
    }

    #[test]
    fn recien_pausada_sigue_visible_con_el_limite_completo() {
        let ahora = Instant::now();
        assert_eq!(visibilidad(Some(false), Some(ahora), ahora, LIMITE), Visibilidad::VisibleDurante(LIMITE));
    }

    #[test]
    fn pausada_un_minuto_visible_dos_minutos_mas() {
        let ahora = Instant::now();
        assert_eq!(
            visibilidad(Some(false), hace(ahora, 60), ahora, LIMITE),
            Visibilidad::VisibleDurante(Duration::from_secs(120))
        );
    }

    #[test]
    fn pasados_tres_minutos_se_oculta() {
        let ahora = Instant::now();
        assert_eq!(visibilidad(Some(false), hace(ahora, 180), ahora, LIMITE), Visibilidad::Oculta);
        assert_eq!(visibilidad(Some(false), hace(ahora, 600), ahora, LIMITE), Visibilidad::Oculta);
    }

    #[test]
    fn al_volver_a_sonar_reaparece() {
        // tras ocultarse, `pausada_desde` se borra al reproducir y vuelve a ser visible
        assert_eq!(visibilidad(Some(true), None, Instant::now(), LIMITE), Visibilidad::Visible);
    }
}
