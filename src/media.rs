//! Lectura y control de la sesión multimedia de Windows (SMTC): Spotify, navegadores, etc.
//! Todo por eventos; los datos quedan en `PISTA` y se avisa a la ventana con `WM_APP`.

use std::ffi::c_void;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Graphics::Imaging::{
    BitmapAlphaMode, BitmapDecoder, BitmapInterpolationMode, BitmapPixelFormat, BitmapTransform,
    ColorManagementMode, ExifOrientationMode,
};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Props,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Estado,
};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

const LADO_PORTADA: u32 = 96; // la portada se reduce al leerla: ~36 KB en memoria
const TICKS_1601_A_1970: i64 = 116_444_736_000_000_000; // DateTime de WinRT cuenta desde 1601

pub struct Portada {
    pub lado: i32,
    pub bgra: Vec<u8>,
}

#[derive(Clone)]
pub struct Pista {
    pub titulo: String,
    pub artista: String,
    pub reproduciendo: bool,
    pub pos_ms: i64, // posición en `desde`; la interfaz la extrapola mientras suena
    pub dur_ms: i64,
    pub desde: Instant,
    pub portada: Option<Arc<Portada>>,
    pub color: (u32, u32, u32), // acento (wave, separador): color de marca de la app o de la portada
}

const ROSA: (u32, u32, u32) = (244, 114, 182);

/// Color de marca por el AppUserModelId de la app que origina la música.
fn color_marca(aumid: &str) -> Option<(u32, u32, u32)> {
    let a = aumid.to_lowercase();
    [
        ("spotify", (30, 215, 96)),
        ("applemusic", (250, 45, 72)),
        ("itunes", (250, 45, 72)),
        ("zunemusic", (0, 120, 212)), // Reproductor multimedia de Windows
        ("vlc", (255, 136, 0)),
        ("youtube", (255, 48, 48)),
        ("deezer", (162, 56, 255)),
        ("amazonmusic", (37, 209, 218)),
    ]
    .iter()
    .find(|(k, _)| a.contains(k))
    .map(|(_, c)| *c)
}

/// Color dominante de la portada (ponderado por saturación y brillo); None si es casi gris.
fn color_dominante(p: &Portada) -> Option<(u32, u32, u32)> {
    let (mut acc, mut peso) = ([0f64; 3], 0f64);
    for px in p.bgra.chunks_exact(4) {
        let c = [px[2] as f64, px[1] as f64, px[0] as f64];
        let (mx, mn) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
        let w = if mx > 0.0 { (mx - mn) / mx * (mx / 255.0) } else { 0.0 };
        (0..3).for_each(|i| acc[i] += w * c[i]);
        peso += w;
    }
    if peso < 20.0 {
        return None;
    }
    let m = acc.iter().fold(0f64, |a, &v| a.max(v / peso));
    let k = if m < 220.0 { 220.0 / m } else { 1.0 }; // se aviva: un acento oscuro no se vería sobre negro
    let f = |i: usize| ((acc[i] / peso * k).min(255.0)) as u32;
    Some((f(0), f(1), f(2)))
}

pub enum Accion {
    Anterior,
    PlayPausa,
    Siguiente,
}

pub static PISTA: Mutex<Option<Pista>> = Mutex::new(None);
/// Cuándo apareció el texto actual: reinicia el desplazamiento solo si cambia la canción.
pub static INICIO: Mutex<Option<Instant>> = Mutex::new(None);
static GESTOR: Mutex<Option<Manager>> = Mutex::new(None);
/// Portada de la canción actual. SMTC publica la imagen con retraso y al principio puede devolver
/// la de la canción anterior: no se da por buena hasta que llega y difiere de la anterior.
struct CachePortada {
    clave: String,
    portada: Option<Arc<Portada>>,
    hash: u64,
    previo: u64, // hash de la portada de la canción anterior
    intentos: u8,
}
static PORTADA: Mutex<CachePortada> =
    Mutex::new(CachePortada { clave: String::new(), portada: None, hash: 0, previo: 0, intentos: 0 });

fn portada_de(p: &Props, clave: &str) -> Option<Arc<Portada>> {
    let previo = {
        let mut c = PORTADA.lock().unwrap();
        if c.clave != clave {
            c.previo = c.hash;
            *c = CachePortada { clave: clave.to_string(), portada: None, hash: 0, previo: c.hash, intentos: 0 };
        }
        if let Some(a) = &c.portada {
            return Some(a.clone());
        }
        c.previo
    };
    let nueva = cargar_portada(p); // sin el candado: decodificar es lento
    let mut c = PORTADA.lock().unwrap();
    if c.clave != clave {
        return None; // cambió de canción mientras cargaba
    }
    c.intentos = c.intentos.saturating_add(1);
    let n = nueva?;
    let mut h = DefaultHasher::new();
    n.bgra.hash(&mut h);
    let h = h.finish();
    // misma imagen que la anterior = SMTC aún no la actualizó; tras varios intentos se acepta (mismo álbum)
    if h == previo && c.intentos <= 4 {
        return None;
    }
    let a = Arc::new(n);
    (c.hash, c.portada) = (h, Some(a.clone()));
    Some(a)
}

fn ms(t: TimeSpan) -> i64 {
    t.Duration / 10_000
}

fn cargar_portada(p: &Props) -> Option<Portada> {
    let flujo = p.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
    let dec = BitmapDecoder::CreateAsync(&flujo).ok()?.join().ok()?;
    let t = BitmapTransform::new().ok()?;
    t.SetScaledWidth(LADO_PORTADA).ok()?;
    t.SetScaledHeight(LADO_PORTADA).ok()?;
    t.SetInterpolationMode(BitmapInterpolationMode::Fant).ok()?;
    let datos = dec
        .GetPixelDataTransformedAsync(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
            &t,
            ExifOrientationMode::IgnoreExifOrientation,
            ColorManagementMode::DoNotColorManage,
        )
        .ok()?
        .join()
        .ok()?;
    Some(Portada { lado: LADO_PORTADA as i32, bgra: datos.DetachPixelData().ok()?.to_vec() })
}

/// Sesión actual si está sonando o en pausa; None si no hay nada.
fn leer(m: &Manager) -> Option<Pista> {
    let s = m.GetCurrentSession().ok()?;
    let estado = s.GetPlaybackInfo().ok()?.PlaybackStatus().ok()?;
    if estado != Estado::Playing && estado != Estado::Paused {
        return None;
    }
    let reproduciendo = estado == Estado::Playing;
    let p = s.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let (titulo, artista) = (p.Title().ok()?.to_string(), p.Artist().ok()?.to_string());

    let tl = s.GetTimelineProperties().ok()?;
    let ini = ms(tl.StartTime().ok()?);
    let dur_ms = (ms(tl.EndTime().ok()?) - ini).max(0);
    let mut pos_ms = ms(tl.Position().ok()?) - ini;
    if reproduciendo {
        // la posición está medida en LastUpdatedTime: se suma lo transcurrido desde entonces
        let ahora = (SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_nanos() / 100) as i64 + TICKS_1601_A_1970;
        pos_ms += ((ahora - tl.LastUpdatedTime().ok()?.UniversalTime) / 10_000).max(0);
    }
    let pos_ms = pos_ms.clamp(0, dur_ms.max(0));

    let portada = portada_de(&p, &format!("{titulo}\u{1}{artista}"));
    let aumid = s.SourceAppUserModelId().map(|x| x.to_string()).unwrap_or_default();
    let color = color_marca(&aumid)
        .or_else(|| portada.as_ref().and_then(|p| color_dominante(p)))
        .unwrap_or(ROSA);
    Some(Pista { titulo, artista, reproduciendo, pos_ms, dur_ms, desde: Instant::now(), portada, color })
}

fn actualizar(m: &Manager, hwnd: isize) {
    let nueva = leer(m);
    {
        let mut actual = PISTA.lock().unwrap();
        let igual = match (&*actual, &nueva) {
            (Some(a), Some(b)) => a.titulo == b.titulo && a.artista == b.artista,
            (None, None) => true,
            _ => false,
        };
        if !igual {
            *INICIO.lock().unwrap() = Some(Instant::now());
            if nueva.is_some() {
                reintentar_portada(m.clone(), hwnd);
            }
        }
        *actual = nueva;
    }
    unsafe {
        let _ = PostMessageW(Some(HWND(hwnd as *mut c_void)), WM_APP, WPARAM(0), LPARAM(0));
    }
}

/// La portada llega tarde: tras un cambio de canción se vuelve a leer unas pocas veces hasta tenerla.
fn reintentar_portada(m: Manager, hwnd: isize) {
    std::thread::spawn(move || {
        // Spotify a veces publica la portada con mucho retraso (medido: más de 14 s); un intento cada
        // 30 s como máximo durante ~2 min, y el hilo termina al tenerla.
        for espera in [300u64, 700, 1500, 3000, 6000, 10_000, 15_000, 20_000, 30_000, 30_000] {
            std::thread::sleep(Duration::from_millis(espera));
            if PISTA.lock().unwrap().as_ref().is_some_and(|p| p.portada.is_some()) {
                break;
            }
            actualizar(&m, hwnd);
        }
    });
}

fn vigilar(m: &Manager, hwnd: isize) {
    actualizar(m, hwnd);
    if let Ok(s) = m.GetCurrentSession() {
        let a = m.clone();
        let _ = s.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
            actualizar(&a, hwnd);
            Ok(())
        }));
        let b = m.clone();
        let _ = s.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
            actualizar(&b, hwnd);
            Ok(())
        }));
        let c = m.clone();
        let _ = s.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
            actualizar(&c, hwnd);
            Ok(())
        }));
    }
}

/// Conecta con Windows y empieza a vigilar la sesión (llamar una vez, con la ventana creada).
pub fn iniciar(hwnd: isize) -> windows::core::Result<()> {
    let m = Manager::RequestAsync()?.join()?;
    vigilar(&m, hwnd);
    let _ = m.CurrentSessionChanged(&TypedEventHandler::new(move |m: windows::core::Ref<Manager>, _| {
        if let Some(m) = m.as_ref() {
            vigilar(m, hwnd);
        }
        Ok(())
    }))?;
    *GESTOR.lock().unwrap() = Some(m);
    Ok(())
}

pub fn accion(a: Accion) {
    if let Some(m) = GESTOR.lock().unwrap().as_ref() {
        if let Ok(s) = m.GetCurrentSession() {
            let _ = match a {
                Accion::Anterior => s.TrySkipPreviousAsync(),
                Accion::PlayPausa => s.TryTogglePlayPauseAsync(),
                Accion::Siguiente => s.TrySkipNextAsync(),
            };
        }
    }
}
