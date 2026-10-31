//! Vista de música: portada, título/artista, onda, progreso y controles.

use super::escala::Escala;
use super::gdi::{Fuente, Lienzo};
use super::iconos::{self, Icono};
use super::tema::*;
use super::textos;
use super::vista::{Modo, Vista};
use crate::dominio::control::Accion;
use crate::dominio::{animacion, pista::Pista};
use crate::estado;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{PtInRect, DRAW_TEXT_FORMAT, DT_CENTER, DT_END_ELLIPSIS};

// Estado compacto (px a 96 dpi)
const MARGEN_COMPACTO: i32 = 14;
const PORTADA_COMPACTA: i32 = 24;
const RADIO_PORTADA_COMPACTA: i32 = 6;
const SEPARACION_COMPACTA: i32 = 10;
const HUECO_SEPARADOR: i32 = 8;

// Estado expandido
const MARGEN_EXPANDIDO: i32 = 16;
const PORTADA_EXPANDIDA: i32 = 44;
const RADIO_PORTADA_EXPANDIDA: i32 = 10;
const SEPARACION_EXPANDIDA: i32 = 10;
const PROGRESO_Y: i32 = 80;
const PROGRESO_ALTO: i32 = 4;
const BOTON_LADO: i32 = 40;
const BOTON_SEPARACION: i32 = 72;
const BOTON_CENTRO_Y: i32 = 112;
const ICONO_MEDIO: i32 = 8;
const ICONO_CENTRAL_MEDIO: i32 = 7;

// Onda
const BARRAS: i32 = 5;
const ANCHO_BARRA: i32 = 4;
const SEPARACION_BARRA: i32 = 3;
const ALTO_BARRA_MINIMO: i32 = 5;
const ALTO_BARRA_VARIABLE: i32 = 15;
const RADIO_BARRA: i32 = 2;
const ANCHO_ONDA: i32 = BARRAS * ANCHO_BARRA + (BARRAS - 1) * SEPARACION_BARRA;

const SEPARADOR: &str = "·";
const SIN_FORMATO_EXTRA: DRAW_TEXT_FORMAT = DRAW_TEXT_FORMAT(0);

pub struct VistaMusica {
    pista: Arc<Pista>,
}

impl VistaMusica {
    /// La vista de la pista que suena ahora, si hay alguna.
    pub fn actual() -> Option<Self> {
        estado::pista().map(|pista| Self { pista })
    }

    /// Botones del estado expandido y la acción de cada uno.
    fn botones(ancho: i32, escala: Escala) -> [(RECT, Accion); 3] {
        let (lado, separacion, cy) = (escala.px(BOTON_LADO), escala.px(BOTON_SEPARACION), escala.px(BOTON_CENTRO_Y));
        let en = |cx: i32| RECT { left: cx - lado / 2, top: cy - lado / 2, right: cx + lado / 2, bottom: cy + lado / 2 };
        [
            (en(ancho / 2 - separacion), Accion::Anterior),
            (en(ancho / 2), Accion::PlayPausa),
            (en(ancho / 2 + separacion), Accion::Siguiente),
        ]
    }

    fn boton_en(punto: POINT, area: RECT, escala: Escala) -> Option<Accion> {
        Self::botones(area.right, escala)
            .into_iter()
            .find(|(zona, _)| unsafe { PtInRect(zona, punto).as_bool() })
            .map(|(_, accion)| accion)
    }

    fn dibujar_portada(&self, lienzo: &Lienzo, zona: RECT, radio: i32, lado_diseno: i32) {
        match &self.pista.portada {
            Some(portada) => lienzo.portada(portada, zona, radio),
            None => {
                // SMTC no la entrega: relleno neutro con una nota, que no parece un fallo de carga
                lienzo.rect_lleno(zona, GRIS_SIN_PORTADA, radio);
                let fuente = Fuente { px: lado_diseno * 3 / 5, peso: 400 };
                lienzo.texto("♪", zona, fuente, GRIS_NOTA, DT_CENTER);
            }
        }
    }

    /// Cinco barras del color de la app; bailan mientras suena y quedan bajas en pausa.
    fn dibujar_onda(&self, lienzo: &Lienzo, derecha: i32, centro_y: i32) {
        let e = lienzo.escala;
        let (ancho, separacion) = (e.px(ANCHO_BARRA), e.px(SEPARACION_BARRA));
        let inicio = derecha - (BARRAS * ancho + (BARRAS - 1) * separacion);
        let t = self.pista.medida_en.elapsed().as_secs_f64();
        for i in 0..BARRAS {
            let variable = if self.pista.reproduciendo {
                (e.px(ALTO_BARRA_VARIABLE) as f64 * animacion::fase_barra(t, i as usize)) as i32
            } else {
                0
            };
            let alto = e.px(ALTO_BARRA_MINIMO) + variable;
            let x = inicio + i * (ancho + separacion);
            let zona = RECT { left: x, top: centro_y - alto / 2, right: x + ancho, bottom: centro_y + alto / 2 };
            lienzo.rect_lleno(zona, self.pista.acento, e.px(RADIO_BARRA));
        }
    }

    /// `título · artista` en tres tramos que se desplazan juntos si no caben. Devuelve true mientras se desplazan.
    fn dibujar_titulo_artista_desplazable(&self, lienzo: &Lienzo, zona: RECT) -> bool {
        let e = lienzo.escala;
        let p = &self.pista;
        let hueco = e.px(HUECO_SEPARADOR);
        let anchos = (
            lienzo.medir(&p.titulo, TITULO_COMPACTO),
            lienzo.medir(SEPARADOR, SEPARADOR_COMPACTO),
            lienzo.medir(&p.artista, ARTISTA_COMPACTO),
        );
        let total = anchos.0 + hueco + anchos.1 + hueco + anchos.2;
        let disponible = zona.right - zona.left;
        let sobra = total - disponible;
        let (desplazamiento, moviendose) = if sobra > 0 {
            let (ms, velocidad) = (estado::ms_desde_el_cambio(), e.px(VELOCIDAD_DESPLAZAMIENTO));
            (animacion::desplazamiento(sobra, ms, velocidad), animacion::sigue_moviendose(sobra, ms, velocidad))
        } else {
            (0, false)
        };

        lienzo.recortar(zona);
        let tramo = |x: i32, ancho: i32| RECT { left: x, top: zona.top, right: x + ancho + 4, bottom: zona.bottom };
        let x = zona.left - desplazamiento;
        lienzo.texto(&p.titulo, tramo(x, anchos.0), TITULO_COMPACTO, BLANCO, SIN_FORMATO_EXTRA);
        let x = x + anchos.0 + hueco;
        lienzo.texto(SEPARADOR, tramo(x, anchos.1), SEPARADOR_COMPACTO, p.acento, SIN_FORMATO_EXTRA);
        let x = x + anchos.1 + hueco;
        lienzo.texto(&p.artista, tramo(x, anchos.2), ARTISTA_COMPACTO, GRIS_TEXTO, SIN_FORMATO_EXTRA);
        lienzo.quitar_recorte();
        moviendose
    }

    fn zona_progreso(&self, lienzo: &Lienzo, area: RECT) -> RECT {
        let e = lienzo.escala;
        let (margen, y) = (e.px(MARGEN_EXPANDIDO), e.px(PROGRESO_Y));
        RECT { left: margen, top: y, right: area.right - margen, bottom: y + e.px(PROGRESO_ALTO) }
    }

    fn dibujar_botones(&self, lienzo: &Lienzo, area: RECT) {
        let e = lienzo.escala;
        let centro = |z: RECT| ((z.left + z.right) / 2, (z.top + z.bottom) / 2);
        let [(anterior, _), (central, _), (siguiente, _)] = Self::botones(area.right, e);

        let (cx, cy) = centro(anterior);
        iconos::dibujar(lienzo, Icono::Anterior, cx, cy, e.px(ICONO_MEDIO), BLANCO);
        let (cx, cy) = centro(siguiente);
        iconos::dibujar(lienzo, Icono::Siguiente, cx, cy, e.px(ICONO_MEDIO), BLANCO);

        // play/pausa: círculo blanco con el icono en negro
        lienzo.rect_lleno(central, BLANCO, (central.right - central.left) / 2);
        let icono = if self.pista.reproduciendo { Icono::Pausa } else { Icono::Play };
        let (cx, cy) = centro(central);
        iconos::dibujar(lienzo, icono, cx, cy, e.px(ICONO_CENTRAL_MEDIO), NEGRO);
    }
}

impl Vista for VistaMusica {
    fn huella_estatica(&self, modo: Modo, area: RECT) -> u64 {
        let p = &self.pista;
        let mut hasher = DefaultHasher::new();
        (modo, area.right, area.bottom).hash(&mut hasher);
        (&p.titulo, &p.artista, p.reproduciendo).hash(&mut hasher);
        p.portada.as_ref().map(|portada| Arc::as_ptr(portada) as usize).hash(&mut hasher);
        hasher.finish()
    }

    fn pintar_estatico(&self, lienzo: &Lienzo, area: RECT, modo: Modo) {
        let e = lienzo.escala;
        match modo {
            Modo::Minimal => {} // solo la onda, que se mueve
            Modo::Compacto => {
                let (margen, lado) = (e.px(MARGEN_COMPACTO), e.px(PORTADA_COMPACTA));
                let y = (area.bottom - lado) / 2;
                let portada = RECT { left: margen, top: y, right: margen + lado, bottom: y + lado };
                self.dibujar_portada(lienzo, portada, e.px(RADIO_PORTADA_COMPACTA), PORTADA_COMPACTA);
            }
            Modo::Expandido => {
                let (margen, lado) = (e.px(MARGEN_EXPANDIDO), e.px(PORTADA_EXPANDIDA));
                let portada = RECT { left: margen, top: margen, right: margen + lado, bottom: margen + lado };
                self.dibujar_portada(lienzo, portada, e.px(RADIO_PORTADA_EXPANDIDA), PORTADA_EXPANDIDA);

                let separacion = e.px(SEPARACION_EXPANDIDA);
                let izquierda = margen + lado + separacion;
                let derecha = area.right - margen - e.px(ANCHO_ONDA) - separacion;
                let medio = margen + lado / 2;
                let titulo = RECT { left: izquierda, top: margen, right: derecha, bottom: medio + e.px(2) };
                let artista = RECT { left: izquierda, top: medio, right: derecha, bottom: margen + lado };
                lienzo.texto(&self.pista.titulo, titulo, TITULO_EXPANDIDO, BLANCO, DT_END_ELLIPSIS);
                lienzo.texto(&self.pista.artista, artista, ARTISTA_EXPANDIDO, GRIS_TEXTO, DT_END_ELLIPSIS);

                lienzo.rect_lleno(self.zona_progreso(lienzo, area), GRIS_BARRA, e.px(2)); // el fondo de la barra
                self.dibujar_botones(lienzo, area);
            }
        }
    }

    fn pintar_dinamico(&self, lienzo: &Lienzo, area: RECT, modo: Modo) -> bool {
        let e = lienzo.escala;
        match modo {
            Modo::Minimal => {
                let centro_x = (area.left + area.right) / 2;
                self.dibujar_onda(lienzo, centro_x + e.px(ANCHO_ONDA) / 2, area.bottom / 2);
                false
            }
            Modo::Compacto => {
                let (margen, lado) = (e.px(MARGEN_COMPACTO), e.px(PORTADA_COMPACTA));
                self.dibujar_onda(lienzo, area.right - margen, area.bottom / 2);
                let separacion = e.px(SEPARACION_COMPACTA);
                let zona_texto = RECT {
                    left: margen + lado + separacion,
                    top: area.top,
                    right: area.right - margen - e.px(ANCHO_ONDA) - separacion,
                    bottom: area.bottom,
                };
                self.dibujar_titulo_artista_desplazable(lienzo, zona_texto)
            }
            Modo::Expandido => {
                let (margen, lado) = (e.px(MARGEN_EXPANDIDO), e.px(PORTADA_EXPANDIDA));
                self.dibujar_onda(lienzo, area.right - margen, margen + lado / 2);
                let pista = self.zona_progreso(lienzo, area);
                let avance = ((pista.right - pista.left) as f64 * self.pista.progreso()) as i32;
                if avance > 0 {
                    lienzo.rect_lleno(RECT { right: pista.left + avance, ..pista }, BLANCO, e.px(2));
                }
                false
            }
        }
    }

    fn zonas_botones(&self, area: RECT, escala: Escala) -> Vec<(RECT, Accion)> {
        Self::botones(area.right, escala).to_vec()
    }

    fn texto_boton(&self, accion: Accion) -> &'static str {
        let textos = textos::actuales();
        match accion {
            Accion::Anterior => textos.anterior,
            Accion::Siguiente => textos.siguiente,
            Accion::PlayPausa if self.pista.reproduciendo => textos.pausar,
            Accion::PlayPausa => textos.reproducir,
        }
    }

    fn hay_boton_en(&self, punto: POINT, area: RECT, escala: Escala) -> bool {
        Self::boton_en(punto, area, escala).is_some()
    }

    fn pulsar(&self, punto: POINT, area: RECT, escala: Escala) -> bool {
        let Some(accion) = Self::boton_en(punto, area, escala) else { return false };
        estado::enviar(accion);
        true
    }

    fn animando(&self) -> bool {
        self.pista.reproduciendo
    }
}
