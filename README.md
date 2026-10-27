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
| Ocultarse a los 3 min de pausa; reaparece al reproducir | Hecho |
| Icono en la bandeja: ocultar/mostrar, tamaño en reposo (Minimal/Compact), cerrar | Hecho (órdenes y tamaños verificados; menú visual sin verificar) |
| Minimal de música (solo la onda) | Hecho (sin verificar visualmente) |
| Minimal, batería, llamada, descarga, temporizador | Pendiente (ver mockup) |
| Ocultarse con apps a pantalla completa | Pendiente |
| Icono de bandeja (salir, configuración, arranque con Windows) | Pendiente |

## Uso

- Suena algo en Windows → la isla aparece sola (también en pausa).
- **Clic** en la isla → se expande. En el Expanded, los botones controlan la reproducción; un **clic en
  cualquier otra zona de la isla, o en cualquier lugar fuera de ella**, la vuelve a compactar.
- **Pausa o parada** → la isla sigue visible **3 minutos** y luego se oculta del todo (no se ve). No
  reaparece por eventos de la canción en pausa: solo cuando vuelve a **reproducir**. Sin sesión multimedia
  (p. ej. cerraste Spotify) se oculta al instante, porque no hay nada que mostrar.
- **Icono de la bandeja** (junto al reloj; en Windows 11 puede quedar en el menú `^` de iconos ocultos):
  clic izquierdo = ocultar/mostrar; clic derecho = menú con **Ocultar/Mostrar isla**, **Tamaño en reposo**
  (Minimal o Compact) y **Cerrar**. El Expanded no se elige: se abre con un clic desde Minimal o Compact y al
  compactar vuelve al tamaño elegido.
- **Ocultar a mano** es independiente de lo anterior: oculta aunque suene música, hasta que la muestres de nuevo
  (si no suena nada, "Mostrar" no la hace aparecer: sigue mandando la regla de los 3 minutos).

## Requisitos para compilar

- Rust (toolchain `stable-x86_64-pc-windows-msvc`).
- Visual Studio con la carga **Desarrollo para el escritorio con C++** (enlazador MSVC + Windows SDK).

```powershell
cargo build --release
.\target\release\isla_win.exe
```

Cierra la isla antes de recompilar: Windows no deja reemplazar un `.exe` en uso.

## Arquitectura

Capas: cada una solo conoce a las de abajo, y la interfaz nunca conoce a SMTC.

```
src/
├── main.rs            arranque: DPI, WinRT, ventana, SMTC, bucle de mensajes
├── dominio/           reglas puras, sin Windows (se prueban con `cargo test`)
│   ├── pista.rs       Pista, Portada, progreso
│   ├── color.rs       color de marca por app y color dominante de la portada
│   ├── animacion.rs   desplazamiento del título y fase de la onda
│   ├── visibilidad.rs cuándo se ve la isla (suena / pausa / 3 min de inactividad)
│   ├── preferencias.rs lo que elige el usuario (tamaño en reposo, ocultar a mano)
│   └── control.rs     Accion y el trait ControlMedios
├── estado.rs          lo que suena + el control: único punto de contacto ui ↔ smtc
├── smtc/              origen de datos (sesión multimedia de Windows)
│   ├── mod.rs         vigila eventos, notifica con un callback, implementa ControlMedios
│   ├── lectura.rs     sesión SMTC → Pista
│   └── portada.rs     lectura de la portada con reintentos y descarte de la anterior
└── ui/
    ├── ventana.rs     ventana: tamaño, forma, temporizador, reacción a mensajes
    ├── vista.rs       trait Vista (contrato) y `activa()` (qué vista mostrar)
    ├── vista_musica.rs  la vista de música (compacta y expandida)
    ├── gdi.rs         primitivas de dibujo (todo el `unsafe` del dibujo está aquí)
    ├── iconos.rs      iconos de control
    ├── bandeja.rs     icono junto al reloj, su menú y su icono dibujado en código
    ├── gancho.rs      gancho de ratón para el clic fuera
    ├── tema.rs        medidas y colores
    └── escala.rs      escala por DPI
```

Principios aplicados:

- **Una responsabilidad por módulo**: la ventana decide tamaño y repintado; la vista decide qué dibujar; `gdi`
  solo sabe dibujar; `smtc` solo sabe leer Windows.
- **Abierto a extensión**: una vista nueva (batería, descarga, llamada) es un archivo que implementa `Vista` y se
  elige en `vista::activa()`, sin tocar la ventana. Una marca nueva de color es una línea en `MARCAS`.
- **Dependencias hacia el dominio**: `ui` y `smtc` dependen de `dominio`; `smtc` avisa a la interfaz con un
  callback que le pasa `main`, no importa nada de `ui`. La interfaz da órdenes al reproductor por el trait
  `ControlMedios`, no por SMTC.
- **Sin números mágicos**: medidas, colores y tiempos tienen nombre (`tema.rs` y las constantes de cada vista).
- **`unsafe` acotado**: los wrappers (`Lienzo`, `gancho`) lo encapsulan; las vistas no lo usan.

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
- **Color de acento** (onda y punto): sale del `SourceAppUserModelId` de la sesión con una tabla de marcas
  (Spotify verde, Apple Music, Reproductor de Windows, VLC, YouTube, Deezer, Amazon Music). Si la app no está
  (p. ej. un navegador) se usa el color dominante de la portada, avivado para que se vea sobre el negro; si no
  hay ninguno, rosa.
- **Clic fuera de la isla**: la ventana no tiene el foco (para no robárselo a otras apps), así que no ve esos
  clics. Mientras está expandida hay un gancho de ratón global (`WH_MOUSE_LL`) que solo avisa de los clics
  fuera de la ventana para compactarla; no consume clics y se quita al compactar.
- **Cursor**: la clase de ventana lleva la flecha (sin cursor de clase Windows deja el de "ocupado", el círculo
  azul) y `WM_SETCURSOR` pone la manita sobre la isla compacta y los botones del Expanded.
- Binario sin runtime: Rust + `windows-rs`, perfil `release` optimizado en tamaño.

## Pruebas

```powershell
cargo test
```

28 pruebas sobre el dominio, la escala y el icono: progreso, desplazamiento del título, colores de marca y
dominante, visibilidad por inactividad, preferencias, órdenes del menú, píxeles del icono, escala por DPI. Lo que toca Windows (ventana, SMTC, dibujo) se verifica a mano con capturas.

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
- Las preferencias (tamaño en reposo, ocultar) viven en memoria: se pierden al cerrar. Tampoco hay arranque
  con Windows todavía.
- Si el Explorador se reinicia, el icono de la bandeja no se vuelve a crear (falta escuchar `TaskbarCreated`).
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
