//! Primitivas de dibujo GDI. Todo el `unsafe` del dibujo queda aquí; el resto de la interfaz usa `Lienzo`.

use super::escala::Escala;
use super::tema::FUENTE;
use crate::dominio::color::Rgb;
use crate::dominio::pista::Portada;
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
            let nombre: Vec<u16> = FUENTE.encode_utf16().chain([0]).collect();
            let hfuente = CreateFontW(
                -self.escala.px(fuente.px), 0, 0, 0, fuente.peso, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, 0, PCWSTR(nombre.as_ptr()),
            );
            let previa = SelectObject(self.hdc, hfuente.into());
            let resultado = usar();
            SelectObject(self.hdc, previa);
            let _ = DeleteObject(hfuente.into());
            resultado
        }
    }
}

fn utf16(texto: &str) -> Vec<u16> {
    texto.encode_utf16().collect()
}

/// Dibuja con doble búfer (sin parpadeo): `dibujar` pinta en memoria y el resultado se copia de una vez.
pub fn pintar_con_doble_buffer<T>(
    hdc: HDC,
    ancho: i32,
    alto: i32,
    escala: Escala,
    dibujar: impl FnOnce(&Lienzo) -> T,
) -> T {
    unsafe {
        let memoria = CreateCompatibleDC(Some(hdc));
        let bitmap = CreateCompatibleBitmap(hdc, ancho, alto);
        let bitmap_previo = SelectObject(memoria, bitmap.into());
        let fondo = RECT { left: 0, top: 0, right: ancho, bottom: alto };
        FillRect(memoria, &fondo, HBRUSH(GetStockObject(BLACK_BRUSH).0));
        SetBkMode(memoria, TRANSPARENT);

        let resultado = dibujar(&Lienzo::nuevo(memoria, escala));

        let _ = BitBlt(hdc, 0, 0, ancho, alto, Some(memoria), 0, 0, SRCCOPY);
        SelectObject(memoria, bitmap_previo);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memoria);
        resultado
    }
}
