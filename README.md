# isla_win

Isla estilo "Dynamic Island" para portátiles Windows cuya cámara está en el **marco**, no en la pantalla.
Una ventana pegada al borde superior, centrada, que muestra lo que suena (Spotify, navegadores…) y se
expande para controlarlo.

> Aquí solo está el **desarrollo**. El diseño y la documentación viven en la carpeta local `mockup_ds`
> (hermana de esta, fuera de este repo).

## Estado

| Pieza | Estado |
|---|---|
| Compact de música (portada, título que se desplaza, wave) | Hecho |
| Expanded (portada, título, artista, progreso, anterior / play-pausa / siguiente) | Hecho, controles verificados con Spotify |
| Ocultarse cuando no hay sesión multimedia | Hecho |
| Minimal, batería, llamada, descarga, temporizador | Pendiente (ver mockup) |
| Ocultarse con apps a pantalla completa | Pendiente |
| Icono de bandeja (salir, configuración, arranque con Windows) | Pendiente |

## Uso

- Suena algo en Windows → la isla aparece sola (también en pausa).
- **Clic** en la isla → se expande. En el Expanded, los botones controlan la reproducción; un **clic en
  cualquier otra zona de la isla, o en cualquier lugar fuera de ella**, la vuelve a compactar.
- Sin sesión multimedia → se oculta.
- Para cerrarla (aún no hay bandeja): `Stop-Process -Name isla_win`.

## Requisitos para compilar

- Rust (toolchain `stable-x86_64-pc-windows-msvc`).
- Visual Studio con la carga **Desarrollo para el escritorio con C++** (enlazador MSVC + Windows SDK).

```powershell
cargo build --release
.\target\release\isla_win.exe
```

Cierra la isla antes de recompilar: Windows no deja reemplazar un `.exe` en uso.

## Arquitectura

- `src/main.rs`: ventana (siempre encima, sin foco, fuera de Alt-Tab, DPI por monitor), dibujo GDI con
  doble búfer, clics y temporizador.
- `src/media.rs`: lectura y control de la sesión multimedia con SMTC
  (`GlobalSystemMediaTransportControls`). Guarda la pista en `PISTA` y avisa a la ventana con `WM_APP`.

Decisiones que mantienen el consumo bajo:

- **Cero sondeo**: todo llega por eventos de Windows (propiedades, estado, línea de tiempo, cambio de sesión).
- **Temporizador solo si hace falta** (50 ms): corre mientras suena algo o el título se desplaza; en pausa
  y sin desplazamiento se apaga.
- La portada se reduce a 96×96 al leerla y se cachea por canción. SMTC la publica con retraso (y al
  principio puede devolver la de la canción anterior), así que no se da por buena hasta que llega y difiere
  de la anterior: tras un cambio de canción se vuelve a leer a los 0,3 / 1 / 2,5 / 5,5 / 11,5 s y luego
  cada 15–30 s hasta ~2 min (un hilo que termina al tenerla). Spotify a veces tarda más de 14 s en publicarla
  (medido: `Thumbnail()` nulo todo ese tiempo). Mientras tanto se muestra un relleno gris con una nota,
  nunca la portada equivocada.
- **Texto del Compact**: `título · artista` en tres tramos (título blanco en negrita, punto de acento,
  artista gris), dibujados uno tras otro para que se desplacen juntos.
- **Color de acento** (wave y punto): sale del `SourceAppUserModelId` de la sesión SMTC con una tabla de marcas
  (Spotify verde, Apple Music, Reproductor de Windows, VLC, YouTube, Deezer, Amazon Music). Si la app no está
  en la tabla (p. ej. un navegador) se usa el color dominante de la portada, avivado para que se vea sobre el
  negro; si no hay ninguno, rosa. Para añadir una marca basta una línea en `color_marca` (`media.rs`).
- **Cursor**: la clase de ventana lleva la flecha (sin cursor de clase Windows deja el de "ocupado", el círculo
  azul) y `WM_SETCURSOR` pone la manita sobre la isla compacta y los botones del Expanded.
- **Clic fuera de la isla**: la ventana no tiene el foco (para no robárselo a otras apps), así que no ve esos
  clics. Mientras está expandida hay un gancho de ratón global (`WH_MOUSE_LL`) que solo avisa de los clics
  fuera de la ventana para compactarla; no consume clics y se quita al compactar.
- Binario sin runtime: Rust + `windows-rs`, perfil `release` optimizado en tamaño.

## Mediciones (orientativas, no un benchmark)

Equipo del autor, pantalla 1536×864 al 125 %, Spotify sonando:

| Situación | RAM | Memoria privada | CPU |
|---|---|---|---|
| Reposo, sin música | ~16 MB | ~2 MB | ~0 % |
| Compact, sonando, título desplazándose | ~17–18 MB | ~2,6 MB | ~1 % de un núcleo |
| Expanded, sonando | ~21 MB | ~3 MB | ~1,6 % de un núcleo (0,08 s en 5 s) |

El `.exe` pesa ~142 KB.

## Atajos conocidos

- Bordes sin antialiasing y sin las curvas inversas del mockup: es GDI con `SetWindowRgn`. Si se adopta,
  pasar a DirectComposition.
- Las suscripciones SMTC se acumulan cuando cambia la sesión actual; des-suscribir si se adopta.
- Sin transición suave al expandir/compactar ni al cambiar de canción: el tamaño cambia de golpe.
- Una pista en pausa sigue visible hasta que la sesión desaparece.
- Posición única: centrada en la pantalla principal (`SM_CXSCREEN`); sin selector de monitor.
- No se probó en un portátil con la cámara real ni con pantalla LCD vs OLED (el negro puede no coincidir
  con el marco).

## Límites de la plataforma (afectan a futuros contenidos)

- **Descargas**: desde fuera del navegador no se conoce el tamaño total; se podría mostrar MB, velocidad y
  una barra indeterminada, no el porcentaje (sin extensión de navegador).
- **Llamadas (Teams/Zoom)**: no hay API pública para silenciar ni colgar; la tarjeta sería informativa.

## Capturas

`capturas/` guarda localmente las capturas de verificación (compact, expanded, pausa, reanudación,
desplazamiento). No se versiona: está en `.gitignore`.

## Histórico

- Prototipo 1: solo texto "título — artista" con puntos suspensivos; cerraba la app al pintar con texto vacío
  (`DrawTextW` con un buffer vacío).
- Prototipo 2: desplazamiento del título largo con doble búfer.
- Versión 3: Compact + Expanded interactivo con portada, progreso y controles. Fallo conocido: la portada
  se quedaba en el relleno azul al cambiar de canción porque se cacheaba el primer resultado, aunque SMTC
  aún no la hubiera publicado.
- Versión actual: portada con reintentos y descarte de la anterior (capturas locales `cap_portada_*.png`). Verificado
  con siguiente → atrás → siguiente en Spotify: la portada correcta aparece en ~0,3 s.
