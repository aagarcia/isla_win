//! Color de acento de la isla: el de la marca de la app o, si no se conoce, el de la portada.

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rgb(pub u8, pub u8, pub u8);

pub const ROSA: Rgb = Rgb(244, 114, 182);

/// Marcas conocidas: fragmento del AppUserModelId (en minúsculas) → color. Añadir una marca = una línea.
const MARCAS: &[(&str, Rgb)] = &[
    ("spotify", Rgb(30, 215, 96)),
    ("applemusic", Rgb(250, 45, 72)),
    ("itunes", Rgb(250, 45, 72)),
    ("zunemusic", Rgb(0, 120, 212)), // Reproductor multimedia de Windows
    ("vlc", Rgb(255, 136, 0)),
    ("youtube", Rgb(255, 48, 48)),
    ("deezer", Rgb(162, 56, 255)),
    ("amazonmusic", Rgb(37, 209, 218)),
];

/// Por debajo de este peso acumulado la imagen se considera casi gris y no aporta un color.
const PESO_MINIMO: f64 = 20.0;
/// Brillo mínimo del acento: uno oscuro no se vería sobre el fondo negro de la isla.
const BRILLO_MINIMO: f64 = 220.0;

pub fn color_marca(aumid: &str) -> Option<Rgb> {
    let aumid = aumid.to_lowercase();
    MARCAS.iter().find(|(clave, _)| aumid.contains(clave)).map(|(_, color)| *color)
}

/// Color dominante de una imagen BGRA, ponderando cada píxel por saturación y brillo.
pub fn color_dominante(bgra: &[u8]) -> Option<Rgb> {
    let (mut suma, mut peso) = ([0f64; 3], 0f64);
    for px in bgra.chunks_exact(4) {
        let canales = [px[2] as f64, px[1] as f64, px[0] as f64]; // r, g, b
        let (max, min) = (canales[0].max(canales[1]).max(canales[2]), canales[0].min(canales[1]).min(canales[2]));
        let w = if max > 0.0 { (max - min) / max * (max / 255.0) } else { 0.0 };
        canales.iter().enumerate().for_each(|(i, c)| suma[i] += w * c);
        peso += w;
    }
    if peso < PESO_MINIMO {
        return None;
    }
    let medio = suma.map(|s| s / peso);
    let mayor = medio.iter().cloned().fold(0.0, f64::max);
    let avivar = if mayor < BRILLO_MINIMO { BRILLO_MINIMO / mayor } else { 1.0 };
    let canal = |i: usize| (medio[i] * avivar).min(255.0) as u8;
    Some(Rgb(canal(0), canal(1), canal(2)))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn imagen(b: u8, g: u8, r: u8, pixeles: usize) -> Vec<u8> {
        [b, g, r, 255].repeat(pixeles)
    }

    #[test]
    fn reconoce_spotify_por_su_id() {
        assert_eq!(color_marca("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), Some(Rgb(30, 215, 96)));
    }

    #[test]
    fn app_desconocida_no_tiene_color_de_marca() {
        assert_eq!(color_marca("Microsoft.Edge"), None);
    }

    #[test]
    fn una_imagen_gris_no_da_color() {
        assert_eq!(color_dominante(&imagen(128, 128, 128, 100)), None);
    }

    #[test]
    fn una_imagen_roja_da_rojo_avivado() {
        let Rgb(r, g, b) = color_dominante(&imagen(0, 0, 100, 100)).unwrap();
        assert!(r >= 200 && g < 30 && b < 30, "{r} {g} {b}");
    }
}
