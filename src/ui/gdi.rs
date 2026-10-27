//! Primitivas de dibujo GDI. Todo el `unsafe` del dibujo queda aquí; el resto de la interfaz usa `Lienzo`.
//!
//! Para gastar poco al animar: las fuentes se crean una vez y se reutilizan, y la parte del dibujo que no
//! cambia entre fotogramas (portada, textos, botones) se pinta en una capa que se copia en cada fotograma.

use super::escala::Escala;
use super::tema::FUENTE;
use crate::dominio::color::Rgb;
use crate::dominio::pista::Portada;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, POINT, RECT};
use windows::Win32::Graphics::Gdi::*;

#[derive(Clone, Copy)]
pub struct Fuente {
    /// Tamaño en px a 96 dpi.
    pub px: i32,
    pub peso: i32,
}

pub fn colorref(Rgb(r, g, b): Rgb) -> COLORREF {
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}

thread_local! {
    /// Fuentes ya creadas, por (alto en px, peso): son pocas y viven lo que el proceso.
    static FUENTES: RefCell<HashMap<(i32, i32), HFONT>> = RefCell::new(HashMap::new());
    static SUPERFICIES: RefCell<Option<Superficies>> = const { RefCell::new(None) };
}

/// Superficie sobre la que se dibuja.
pub struct Lienzo {
    hdc: HDC,
    pub escala: Escala,
}

impl Lienzo {
    /// # Safety
    /// `hdc` debe ser un contexto válido durante toda la vida del `Lienzo`.
    unsafe fn nuevo(hdc: HDC, escala: Escala) -> Self {
        Self { hdc, escala }
    }

    pub fn rect_lleno(&self, zona: RECT, color: Rgb, radio: i32) {
        self.con_pincel(color, || unsafe {
            // RoundRect con lápiz nulo excluye el borde derecho e inferior: +1 para cubrirlos
            let _ = RoundRect(self.hdc, zona.left, zona.top, zona.right + 1, zona.bottom + 1, radio * 2, radio * 2);
        });
    }

    pub fn poligono(&self, puntos: &[POINT], color: Rgb) {
        self.con_pincel(color, || unsafe {
            let _ = Polygon(self.hdc, puntos);
        });
    }

    /// Dibuja `texto` en una línea dentro de `zona`, centrado en vertical.
    pub fn texto(&self, texto: &str, mut zona: RECT, fuente: Fuente, color: Rgb, extra: DRAW_TEXT_FORMAT) {
        if texto.is_empty() {
            return; // DrawTextW con un buffer vacío cierra el proceso
        }
        self.con_fuente(fuente, || unsafe {
            SetTextColor(self.hdc, colorref(color));
            DrawTextW(self.hdc, &mut utf16(texto), &mut zona, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | extra);
        });
    }

    /// Ancho en px que ocupa `texto` con `fuente`.
    pub fn medir(&self, texto: &str, fuente: Fuente) -> i32 {
        if texto.is_empty() {
            return 0;
        }
        let mut zona = RECT::default();
        self.con_fuente(fuente, || unsafe {
            DrawTextW(self.hdc, &mut utf16(texto), &mut zona, DT_SINGLELINE | DT_NOPREFIX | DT_CALCRECT);
        });
        zona.right - zona.left
    }

    /// Limita el dibujo a `zona` hasta llamar a `quitar_recorte`.
    pub fn recortar(&self, zona: RECT) {
        unsafe {
            let _ = IntersectClipRect(self.hdc, zona.left, zona.top, zona.right, zona.bottom);
        }
    }

    pub fn quitar_recorte(&self) {
        unsafe {
            SelectClipRgn(self.hdc, None);
        }
    }

    /// Dibuja la portada escalada a `zona`, con esquinas redondeadas.
    pub fn portada(&self, portada: &Portada, zona: RECT, radio: i32) {
        unsafe {
            let region = CreateRoundRectRgn(zona.left, zona.top, zona.right + 1, zona.bottom + 1, radio * 2, radio * 2);
            SelectClipRgn(self.hdc, Some(region));
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: portada.lado,
                    biHeight: -portada.lado, // filas de arriba abajo
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            SetStretchBltMode(self.hdc, HALFTONE);
            StretchDIBits(
                self.hdc, zona.left, zona.top, zona.right - zona.left, zona.bottom - zona.top,
                0, 0, portada.lado, portada.lado,
                Some(portada.bgra.as_ptr() as *const c_void), &info, DIB_RGB_COLORS, SRCCOPY,
            );
            self.quitar_recorte();
            let _ = DeleteObject(region.into());
        }
    }

    fn con_pincel(&self, color: Rgb, dibujar: impl FnOnce()) {
        unsafe {
            let pincel = CreateSolidBrush(colorref(color));
            let pincel_previo = SelectObject(self.hdc, pincel.into());
            let lapiz_previo = SelectObject(self.hdc, GetStockObject(NULL_PEN));
            dibujar();
            SelectObject(self.hdc, lapiz_previo);
            SelectObject(self.hdc, pincel_previo);
            let _ = DeleteObject(pincel.into());
        }
    }

    fn con_fuente<T>(&self, fuente: Fuente, usar: impl FnOnce() -> T) -> T {
        unsafe {
            let hfuente = fuente_cacheada(-self.escala.px(fuente.px), fuente.peso);
            let previa = SelectObject(self.hdc, hfuente.into());
            let resultado = usar();
            SelectObject(self.hdc, previa);
            resultado
        }
    }
}

/// La fuente de ese alto y peso; solo se crea la primera vez que se pide.
fn fuente_cacheada(alto: i32, peso: i32) -> HFONT {
    FUENTES.with(|fuentes| {
        *fuentes.borrow_mut().entry((alto, peso)).or_insert_with(|| unsafe {
            let nombre: Vec<u16> = FUENTE.encode_utf16().chain([0]).collect();
            CreateFontW(
                alto, 0, 0, 0, peso, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, 0, PCWSTR(nombre.as_ptr()),
            )
        })
    })
}

fn utf16(texto: &str) -> Vec<u16> {
    texto.encode_utf16().collect()
}

// ---------- superficies en memoria ----------

/// Un contexto de dibujo en memoria con su bitmap.
struct Plano {
    dc: HDC,
    bitmap: HBITMAP,
    bitmap_previo: HGDIOBJ,
}

impl Plano {
    fn nuevo(referencia: HDC, ancho: i32, alto: i32) -> Self {
        unsafe {
            let dc = CreateCompatibleDC(Some(referencia));
            let bitmap = CreateCompatibleBitmap(referencia, ancho, alto);
            let bitmap_previo = SelectObject(dc, bitmap.into());
            SetBkMode(dc, TRANSPARENT);
            Self { dc, bitmap, bitmap_previo }
        }
    }

    fn rellenar_de_negro(&self, ancho: i32, alto: i32) {
        unsafe {
            let todo = RECT { left: 0, top: 0, right: ancho, bottom: alto };
            FillRect(self.dc, &todo, HBRUSH(GetStockObject(BLACK_BRUSH).0));
        }
    }
}

impl Drop for Plano {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.bitmap_previo);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

/// Las dos superficies que se reutilizan entre fotogramas (en vez de crearlas en cada uno).
struct Superficies {
    ancho: i32,
    alto: i32,
    /// Huella de lo que contiene la capa estática; si cambia, se vuelve a pintar.
    huella: Option<u64>,
    capa_estatica: Plano,
    fotograma: Plano,
}

/// Pinta un fotograma sin parpadeo y gastando poco:
/// 1. la capa estática solo se vuelve a pintar si cambia su `huella` (o el tamaño);
/// 2. cada fotograma copia esa capa, pinta encima lo que se mueve (`dinamico`) y lo vuelca a la pantalla.
pub fn pintar_en_capas<T>(
    hdc: HDC,
    ancho: i32,
    alto: i32,
    escala: Escala,
    huella: u64,
    estatico: impl FnOnce(&Lienzo),
    dinamico: impl FnOnce(&Lienzo) -> T,
) -> T {
    SUPERFICIES.with(|celda| {
        let mut celda = celda.borrow_mut();
        let cambio_de_tamano = celda.as_ref().is_none_or(|s| s.ancho != ancho || s.alto != alto);
        if cambio_de_tamano {
            *celda = Some(Superficies {
                ancho,
                alto,
                huella: None,
                capa_estatica: Plano::nuevo(hdc, ancho, alto),
                fotograma: Plano::nuevo(hdc, ancho, alto),
            });
        }
        let s = celda.as_mut().unwrap();
        unsafe {
            if s.huella != Some(huella) {
                s.capa_estatica.rellenar_de_negro(ancho, alto);
                estatico(&Lienzo::nuevo(s.capa_estatica.dc, escala));
                s.huella = Some(huella);
            }
            let _ = BitBlt(s.fotograma.dc, 0, 0, ancho, alto, Some(s.capa_estatica.dc), 0, 0, SRCCOPY);
            let resultado = dinamico(&Lienzo::nuevo(s.fotograma.dc, escala));
            let _ = BitBlt(hdc, 0, 0, ancho, alto, Some(s.fotograma.dc), 0, 0, SRCCOPY);
            resultado
        }
    })
}
