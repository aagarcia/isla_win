//! Cálculos de animación sin dibujo: pura matemática, fácil de probar.

/// Pausa en cada extremo del desplazamiento del título (ms).
const PAUSA_MS: i64 = 1500;
/// Veces que se recorre el título antes de dejarlo quieto: moverlo para siempre gastaría CPU sin aportar.
const CICLOS: i64 = 2;

fn duracion_del_ciclo(distancia: i64, velocidad: i64) -> i64 {
    2 * (PAUSA_MS + distancia * 1000 / velocidad)
}

/// Desplazamiento (px) de un texto más largo que su hueco: pausa, avanza despacio, pausa y vuelve.
/// `distancia` es lo que sobra del texto; `ms`, el tiempo desde que apareció; `velocidad`, en px/s.
pub fn desplazamiento(distancia: i32, ms: i64, velocidad: i32) -> i32 {
    let (distancia, velocidad) = (distancia as i64, velocidad.max(1) as i64);
    if !sigue_moviendose(distancia as i32, ms, velocidad as i32) {
        return 0;
    }
    let recorrido = distancia * 1000 / velocidad; // ms que tarda en recorrer la distancia
    let t = ms % duracion_del_ciclo(distancia, velocidad);
    let desplazado = if t < PAUSA_MS {
        0
    } else if t < PAUSA_MS + recorrido {
        (t - PAUSA_MS) * velocidad / 1000
    } else if t < 2 * PAUSA_MS + recorrido {
        distancia
    } else {
        distancia - (t - 2 * PAUSA_MS - recorrido) * velocidad / 1000
    };
    desplazado.clamp(0, distancia) as i32
}

/// ¿Sigue desplazándose el título o ya terminó sus ciclos y quedó quieto?
pub fn sigue_moviendose(distancia: i32, ms: i64, velocidad: i32) -> bool {
    ms < CICLOS * duracion_del_ciclo(distancia as i64, velocidad.max(1) as i64)
}

/// Altura relativa (0..1) de la barra `indice` de la onda en el instante `t` (segundos).
pub fn fase_barra(t: f64, indice: usize) -> f64 {
    0.5 + 0.5 * (t * 6.0 + indice as f64 * 1.3).sin()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn empieza_quieto() {
        assert_eq!(desplazamiento(100, 0, 50), 0);
    }

    #[test]
    fn avanza_tras_la_pausa_inicial() {
        assert_eq!(desplazamiento(100, PAUSA_MS + 1000, 50), 50);
    }

    #[test]
    fn se_detiene_al_final() {
        let recorrido = 100 * 1000 / 50;
        assert_eq!(desplazamiento(100, PAUSA_MS + recorrido + 10, 50), 100);
    }

    #[test]
    fn vuelve_al_principio_y_repite() {
        let ciclo = 2 * (PAUSA_MS + 2000);
        assert_eq!(desplazamiento(100, ciclo, 50), 0);
    }

    #[test]
    fn tras_dos_ciclos_se_queda_quieto() {
        let ciclo = 2 * (PAUSA_MS + 2000);
        assert!(sigue_moviendose(100, ciclo + 10, 50));
        assert!(!sigue_moviendose(100, 2 * ciclo, 50));
        assert_eq!(desplazamiento(100, 2 * ciclo + PAUSA_MS + 1000, 50), 0);
    }

    #[test]
    fn la_onda_queda_entre_0_y_1() {
        assert!((0..100).all(|i| (0.0..=1.0).contains(&fase_barra(i as f64 * 0.1, i % 5))));
    }
}
