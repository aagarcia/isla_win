# isla_win

[Español](README.md) · **English**

"Dynamic Island"-style widget for Windows laptops whose camera sits in the **bezel**, not on the screen.
A window attached to the top edge, centered, that shows what is playing (Spotify, browsers…) and expands
to control it.

> Only the **development** lives here. Design and documentation live in the local folder `mockup_ds`
> (a sibling of this one, outside this repo).

## Status

| Piece | Status |
|---|---|
| Music Compact (cover, scrolling title, wave) | Done |
| Expanded (cover, title, artist, progress, previous / play-pause / next) | Done, controls verified with Spotify |
| Hide after 3 min of pause; reappears on play | Done |
| Tray icon: hide/show, resting size (Minimal/Compact), close | Done (commands and sizes verified; visual menu unverified) |
| Music Minimal (wave only) | Done (not visually verified) |
| Battery, call, download, timer | Pending (see mockup) |
| Hide with full-screen apps | Pending |
| Tray icon (settings, start with Windows) | Pending |
| English / Spanish UI (follows the Windows language) | Done |

## Usage

- Something plays in Windows → the island appears on its own (also while paused).
- **Click** the island → it expands. In Expanded, the buttons control playback; a **click on any other area
  of the island, or anywhere outside it**, compacts it again.
- **Pause or stop** → the island stays visible for **3 minutes** and then hides completely. It does not
  reappear on events from the paused song: only when it **plays** again. With no media session
  (e.g. you closed Spotify) it hides immediately, since there is nothing to show.
- **Tray icon** (next to the clock; on Windows 11 it may sit in the hidden-icons `^` menu):
  left click = hide/show; right click = menu with **Hide/Show island**, **Resting size**
  (Minimal or Compact), **Language** (Español / English) and **Close**. Expanded is not a choice: it opens with a click from Minimal or Compact
  and returns to the chosen size when compacted.
- **Hiding by hand** is independent of the above: it hides even while music plays, until you show it again
  (if nothing is playing, "Show" does not make it appear: the 3-minute rule still applies).
- **Language**: at startup, Spanish if Windows' display language is Spanish, English otherwise. Change it
  from the tray menu (**Language**); the choice lives in memory and is lost on close.

## Build requirements

- Rust (toolchain `stable-x86_64-pc-windows-msvc`).
- Visual Studio with the **Desktop development with C++** workload (MSVC linker + Windows SDK).

```powershell
cargo build --release
.\target\release\isla_win.exe
```

Close the island before rebuilding: Windows won't let you replace an `.exe` that is in use.

## Architecture

Layers: each one only knows the ones below it, and the UI never knows SMTC.

```
src/
├── main.rs            startup: DPI, WinRT, window, SMTC, message loop
├── dominio/           pure rules, no Windows (tested with `cargo test`)
│   ├── pista.rs       Track, Cover, progress
│   ├── color.rs       per-app brand color and cover dominant color
│   ├── animacion.rs   title scrolling and wave phase
│   ├── visibilidad.rs when the island is visible (playing / paused / 3 min idle)
│   ├── preferencias.rs what the user chooses (resting size, hide by hand)
│   └── control.rs     Action and the MediaControl trait
├── estado.rs          what is playing + control: the only ui ↔ smtc contact point
├── smtc/              data source (Windows media session)
│   ├── mod.rs         watches events, notifies through a callback, implements the control trait
│   ├── lectura.rs     SMTC session → Track
│   └── portada.rs     cover reading with retries and discarding the previous one
└── ui/
    ├── ventana.rs     window: size, shape, timer, message handling
    ├── vista.rs       View trait (contract) and `activa()` (which view to show)
    ├── vista_musica.rs  the music view (compact and expanded)
    ├── gdi.rs         drawing primitives, cached fonts and layered painting (all the drawing `unsafe`)
    ├── iconos.rs      control icons
    ├── bandeja.rs     tray icon, its menu and its code-drawn icon
    ├── textos.rs      UI texts in Spanish and English; language taken from Windows
    ├── gancho.rs      mouse hook for click-outside
    ├── tema.rs        measurements and colors
    └── escala.rs      DPI scaling
```

Principles applied:

- **One responsibility per module**: the window decides size and repaint; the view decides what to draw; `gdi`
  only knows how to draw; `smtc` only knows how to read Windows.
- **Open to extension**: a new view (battery, download, call) is a file that implements `Vista` and is
  selected in `vista::activa()`, without touching the window. A new brand color is one line in `MARCAS`.
- **Dependencies point to the domain**: `ui` and `smtc` depend on `dominio`; `smtc` notifies the UI through
  a callback passed by `main`, and imports nothing from `ui`. The UI sends commands to the player through the
  `ControlMedios` trait, not through SMTC.
- **No magic numbers**: measurements, colors and times have names (`tema.rs` and each view's constants).
- **Bounded `unsafe`**: the wrappers (`Lienzo`, `gancho`) encapsulate it; the views don't use it.

Decisions that keep consumption low:

- **Zero polling**: everything arrives through Windows events (properties, state, timeline, session change).
- **Timer only when needed** (50 ms): it runs while something plays or the title scrolls; when paused
  and not scrolling it is turned off.
- The cover is reduced to 96×96 when read and cached per song. SMTC publishes it late (and at first
  it may return the previous song's), so it is not accepted until it arrives and differs from the previous
  one: after a song change it is re-read at 0.3 / 1 / 2.5 / 5.5 / 11.5 s and then every 15–30 s up to
  ~2 min (a thread that ends once it has it). Spotify sometimes takes over 14 s to publish it
  (measured: `Thumbnail()` null the whole time). Meanwhile a gray placeholder with a note is shown,
  never the wrong cover.
- **Accent color** (wave and dot): taken from the session's `SourceAppUserModelId` with a brand table
  (Spotify green, Apple Music, Windows media player, VLC, YouTube, Deezer, Amazon Music). If the app is not
  there (e.g. a browser) the cover's dominant color is used, brightened to stand out on black; if there is
  none, pink.
- **Click outside the island**: the window has no focus (so it doesn't steal it from other apps), so it
  doesn't see those clicks. While expanded there is a global mouse hook (`WH_MOUSE_LL`) that only reports
  clicks outside the window so it can compact; it doesn't consume clicks and is removed on compacting.
- **Cursor**: the window class carries the arrow (without a class cursor Windows keeps the "busy" one, the
  blue circle) and `WM_SETCURSOR` shows the hand over the compact island and the Expanded buttons.
- Runtime-free binary: Rust + `windows-rs`, `release` profile optimized for size.

## Tests

```powershell
cargo test
```

34 tests on the domain, scale, icon and texts: progress, title scrolling, brand and dominant colors,
visibility on inactivity, preferences, menu commands, icon pixels, DPI scaling, language selection. Whatever
touches Windows (window, SMTC, drawing) is verified by hand with screenshots.

## Measurements (indicative, not a benchmark)

Author's machine (8 logical cores), 1536×864 screen at 125 %, Spotify playing. Clean start; each row is a
15 s window after 3 s of settling. "% of 1 core": 100 % would be one full core.

| State | CPU before | CPU now | RAM / private now |
|---|---|---|---|
| Compact, playing | 1.35 % | **0.83 %** | 22.1 MB / 3.1 MB |
| Minimal, playing | 0.52 % | **0.42 %** | 22.1 MB / 3.1 MB |
| Expanded, playing | 1.35 % | **0.21 %** | 22.3 MB / 3.2 MB |

- **What was optimized** (see "Performance" below): cached fonts, cached static layer, reused surfaces,
  15 fps and bounded scrolling. Expanded drops by ~84 %.
- **No leaks**: bounded GDI objects (21: two surfaces and the cached fonts, they don't grow when changing
  state) and stable RAM (22.1 → 22.3 MB in a minute).
- The **private memory (~3 MB)** is what the island really contributes; the working set includes shared
  Windows DLL pages. For reference, Spotify uses ~630 MB across all its processes.
- Measured before optimizing, after hours open: 27.7 MB of RAM (private 3.6 MB). Still pending: check
  whether the working set grows with hours of use.
- `.exe`: ~180 KB.
- Hidden by inactivity (paused): 0.000 % CPU (measured before the tray). The "paused, visible" state has
  **not been measured properly** (the pause key never actually paused); pending.

## Performance: rules so it doesn't degrade as it grows

- **Nothing is created every frame**: GDI fonts are cached by (size, weight) and the two in-memory
  surfaces are reused while the size doesn't change.
- **Static layer**: each `Vista` separates what doesn't change (cover, texts, buttons) from what moves (wave,
  progress, scrolling title). The static layer is painted once and copied; it is only repainted if its
  fingerprint changes (song, size, play/pause, cover). A new view must respect this separation.
- **Bounded animation**: ~15 fps (66 ms) and a long title scrolls 2 cycles and then stays still; the
  timer is turned off when nothing moves.
- **Shared state without copies**: `estado::pista()` returns an `Arc`; reading it copies neither text nor cover.
- **Nothing accumulates with use**: each new song invalidates the previous cover retry (a single live
  thread) and on SMTC session change the previous handlers are unsubscribed (one event = one read).
- Pending ideas if needed: skip repainting when the wave doesn't visually change, and batch SMTC timeline
  events (they arrive often).

## Known shortcuts

- Edges without antialiasing and without the mockup's inverse curves: it is GDI with `SetWindowRgn`. If
  adopted, move to DirectComposition.
- No smooth transition when expanding/compacting or changing song: the size changes at once.
- Preferences (resting size, hide) live in memory: they are lost on close. There is no start with Windows
  yet either.
- If Explorer restarts, the tray icon is not recreated (we don't listen for `TaskbarCreated`).
- Single position: centered on the primary screen (`SM_CXSCREEN`); no monitor selector.
- Not tested on a laptop with the real camera, nor on LCD vs OLED (the black may not match the bezel).

## Platform limits (affect future content)

- **Downloads**: outside the browser the total size is unknown; MB, speed and an indeterminate bar could be
  shown, but not the percentage (without a browser extension).
- **Calls (Teams/Zoom)**: there is no public API to mute or hang up; the card would be informational.

## Screenshots

`capturas/` keeps the verification screenshots locally (compact, expanded, pause, resume, scrolling). It is
not versioned: it is in `.gitignore`.

## History

- Prototype 1: text only "title — artist" with ellipsis; crashed the app when painting empty text
  (`DrawTextW` with an empty buffer).
- Prototype 2: long-title scrolling with double buffering.
- Version 3: Compact + interactive Expanded with cover, progress and controls. Known bug: the cover stayed
  on the blue placeholder when changing song because the first result was cached, even though SMTC hadn't
  published it yet.
- Current version: cover with retries and discarding the previous one (local screenshots `cap_portada_*.png`).
  Verified with next → back → next on Spotify: the correct cover appears in ~0.3 s.
