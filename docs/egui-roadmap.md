# Rust egui-on-Tizen roadmap

**Goal: render an egui "hello world" UI on Tizen TV via EGL/GLES,
using rust-tizen as the platform layer. No input, no CPU backend —
those come after we hit the goal.**

The target compositor is .234 (Tizen 10/TV, ARM Mali GL stack). Each
example crate consumes `tizen` via a GitHub URL — `cargo tizen build`
must work for anyone with the repo path and a configured rootstrap.

This file is the single source of truth for progress; we tick items
off as work lands.

## Design rule: platform crates stay UI-framework-agnostic

`tizen-window`, `tizen-egl`, and any other platform crate **must not
take a dependency on egui, slint, iced, winit, sctk, or any other UI
framework or windowing abstraction**. They expose only generic
Wayland/GLES/TBM primitives plus the standard ecosystem interop
traits (`raw-window-handle`).

All UI-framework wiring lives in the **example crates** under
`examples/`. So today's example is `hello-egui-gpu`; tomorrow's
might be `hello-slint-gpu` or `hello-iced-gpu`, and each is a thin
glue file using only the platform crates' public API.

This keeps `tizen-window` usable by anyone — not just egui — and
prevents the "the platform crate forces a specific framework on
me" anti-pattern.

### Use what popular crates provide, don't invent

Where there's an existing ecosystem standard, we adopt it directly:

| Concern | What we use |
|---|---|
| Surface discovery for renderers | `raw-window-handle = "0.6"` — `HasDisplayHandle` / `HasWindowHandle` |
| Bitflag modifier state | `bitflags = "2"` — mirror `winit::keyboard::ModifiersState` field names (`SHIFT`, `CTRL`, `ALT`, `LOGO`) |
| Pointer button enum | Variant names match `winit::event::MouseButton` (`Left`, `Right`, `Middle`, `Back`, `Forward`, `Other(u16)`) |

Where there's no universal crate (e.g. an Event enum that all UI
frameworks accept), we **mirror winit's shape** rather than invent a
new vocabulary:

```rust
// tizen-window's Event mirrors winit::event::WindowEvent variants
pub enum Event {
    Resized { width: u32, height: u32 },     // ≈ Resized(PhysicalSize)
    RedrawRequested,                          // = RedrawRequested
    CloseRequested,                           // = CloseRequested

    Focused(bool),                            // = Focused(bool)

    CursorEntered { x: f64, y: f64 },        // ≈ CursorEntered { device_id }
    CursorLeft,                               // ≈ CursorLeft  { device_id }
    CursorMoved  { x: f64, y: f64 },         // ≈ CursorMoved  { device_id, position }
    MouseInput   { button: MouseButton, pressed: bool },
                                              // ≈ MouseInput   { device_id, state, button }
    MouseWheel   { dx: f64, dy: f64 },       // ≈ MouseWheel   { device_id, delta }

    KeyboardInput { keycode: u32, pressed: bool, modifiers: ModifiersState },
                                              // ≈ KeyboardInput { device_id, event, is_synthetic }
}
```

We don't *depend* on `winit` itself (it's a heavy cross-platform
windowing crate that wants to own the window — not relevant for us);
we just match its public type shapes so anyone who already knows
winit needs zero new mental model.

## Status snapshot

What we already have (built in prior sessions):

- `tizen-window` — TBM-backed Wayland window via `zxdg_shell_v6` +
  `wtz_shell` + `tizen_policy`. Renders a solid-colour buffer,
  verified visible on .234.
- `tizen-tbm-sys`, `tizen-input{,-sys}`, `tizen-screenshot{,-sys}`,
  the `tizen` umbrella crate with feature flags. All on
  `smohantty/rust-tizen` `main`.

What's missing to render egui:

```
┌──────────────────────────────────────────────────────────────┐
│ Phase 1 — `tizen-window` foundation (no new crates)          │
│   1.1 ☑ raw-window-handle impls                              │
│   1.2 ☑ wl_surface.frame callback API                        │
│   1.3 ☐ Repaint on configure (compositor resize)             │
│   1.4 ☐ Generic Event enum + Display::run helper             │
│   1.5 ☐ wl_pointer wiring (enter/leave/motion/button/wheel)  │
│   1.6 ☐ wl_keyboard wiring (raw keycodes; xkb deferred)      │
├──────────────────────────────────────────────────────────────┤
│ Phase 2 — `tizen-egl` crate + EGL probe                      │
│   2.1 ☐ tizen-egl-sys (wl_egl_window FFI)                    │
│   2.2 ☐ tizen-egl safe wrapper                               │
│   2.3 ☐ Umbrella `egl` feature                               │
│   2.4 ☐ examples/hello-egl-probe — go/no-go for GLES 3.x     │
├──────────────────────────────────────────────────────────────┤
│ Phase 3 — egui hello world                                   │
│   3.1 ☐ examples/hello-egui-gpu — egui_glow + tizen-egl      │
│   3.2 ☐ Cross-build, deploy, verify visible on .234          │
└──────────────────────────────────────────────────────────────┘
```

## Phase 1 — `tizen-window` foundation

Everything that any renderer needs from us, regardless of CPU/GPU
choice. No new crates; pure additions to `tizen-window`.

- [x] **1.1 raw-window-handle impls.** Added `raw-window-handle = "0.6"`
      dep; `Display` impls `HasDisplayHandle` returning a
      `WaylandDisplayHandle` wrapping the `Connection`'s
      `backend().display_ptr()`; `Window` impls `HasWindowHandle`
      returning a `WaylandWindowHandle` wrapping the
      `wl_surface`'s `id().as_ptr()`. Both lifetimes are tied to
      `&self` via `borrow_raw`. Verified by `cargo check + clippy +
      tests` clean.

- [x] **1.2 Frame callback API.** `Window::request_redraw()` calls
      `wl_surface.frame(&qh, ())` to register a one-shot
      `wl_callback`. The `Dispatch<WlCallback, ()>` impl pushes
      `Event::RedrawRequested` into the pending-events queue when
      `done` fires. Caller drains via `Window::drain_events()`
      (placeholder until `Display::run` in 1.4 hides this).

      Also added the `Event` enum (event.rs, mirroring winit's
      WindowEvent variants) + `MouseButton` + `ModifiersState`
      bitflags. `Resized` + `CloseRequested` are wired through the
      existing `zxdg_toplevel_v6` dispatcher; the rest get filled
      in by Phases 1.5/1.6.

- [ ] **1.3 Repaint on configure.** When `zxdg_toplevel.configure`
      delivers a non-zero (w, h), update internal size and fire the
      redraw callback so the buffer matches. Today we stay at 640×480
      even when the compositor places us at 1920×1080.

- [ ] **1.4 Event loop helper + winit-shaped Event enum.** Bare-bones
      `Display::run(window, callback)` that blocks dispatching events.
      Variant names and field shapes mirror `winit::event::WindowEvent`
      (see Design Rules section above):

  ```rust
  pub enum Event {
      Resized { width: u32, height: u32 },
      RedrawRequested,
      CloseRequested,
      Focused(bool),
      CursorEntered { x: f64, y: f64 },
      CursorLeft,
      CursorMoved   { x: f64, y: f64 },
      MouseInput    { button: MouseButton, pressed: bool },
      MouseWheel    { dx: f64, dy: f64 },
      KeyboardInput { keycode: u32, pressed: bool, modifiers: ModifiersState },
  }
  ```

  `MouseButton`, `ModifiersState` mirror winit's shapes. No egui types
  leak into this enum — the example does the egui translation.

- [ ] **1.5 wl_pointer wiring.** Bind seat → wl_pointer; dispatch
      `enter`, `leave`, `motion`, `button`, `axis` events; deliver
      through the winit-shaped `Event::Cursor*` / `MouseInput` /
      `MouseWheel` variants.

- [ ] **1.6 wl_keyboard wiring (raw keycodes).** Bind seat →
      wl_keyboard; deliver `enter`, `leave`, `modifiers`, `key`. v1
      ships **raw evdev keycodes** in `Event::KeyboardInput {
      keycode, … }` — no xkbcommon, no text-input. egui's
      hello-world demo only needs Esc / Enter / arrow keys to be
      interactive enough, all mappable from raw keycodes. Full
      xkbcommon + UTF-8 text input is post-MVP.

## Phase 2 — `tizen-egl` + capability probe

The single new platform crate. Wraps `libwayland-egl.so.1` (in the
rootstrap, no dlopen needed) and exposes an `EglWindow` that
`khronos-egl` can use to create a window surface.

- [ ] **2.1 `tizen-egl-sys`.** Extern "C" bindings for
      `wl_egl_window_create / _destroy / _resize`. ~30 LOC.
      `#[cfg_attr(tizen, link(name = "wayland-egl", kind = "dylib"))]`.

- [ ] **2.2 `tizen-egl`.** Safe wrapper. Public:

  ```rust
  pub struct EglWindow { /* … */ }
  impl EglWindow {
      pub fn new(window: &tizen_window::Window, w: u32, h: u32) -> Result<Self>;
      pub fn as_egl_ptr(&self) -> *mut c_void;       // for eglCreateWindowSurface
      pub fn resize(&self, w: u32, h: u32, dx: i32, dy: i32);
  }
  // Drop frees the wl_egl_window.
  ```

- [ ] **2.3 Umbrella `egl` feature.** Add `egl = ["window",
      "dep:tizen-egl"]` to `tizen/Cargo.toml`. Document in the
      umbrella's README.

- [ ] **2.4 `hello-egl-probe`.** Tiny example that:
      ① opens a Window, ② creates an EglWindow, ③ initialises EGL,
      ④ binds GLES API, ⑤ creates a context (asks for GLES 3.x first,
      falls back to 2.0), ⑥ prints `eglQueryString(VENDOR /
      CLIENT_APIS / EXTENSIONS)` + `glGetString(VENDOR / RENDERER /
      VERSION / SHADING_LANGUAGE_VERSION)`.

      **Go/no-go gate for Phase 3.** If the probe shows GLES 3.0+,
      egui_glow is on. If only GLES 2.0, we'd need to fall back to
      ancient egui versions and the path is much rougher.

## Phase 3 — egui hello world

The payoff. `examples/hello-egui-gpu/` opens a window, paints an
animated egui UI via the GPU.

- [ ] **3.1 `hello-egui-gpu` crate.** `Cargo.toml`:

  ```toml
  [workspace]
  [package]
  name = "hello-egui-gpu"
  version = "0.1.0"
  edition = "2021"
  publish = false

  [dependencies]
  tizen = { git = "https://github.com/smohantty/rust-tizen.git",
            features = ["window", "egl"] }
  egui = "0.29"
  egui_glow = "0.29"
  glow = "0.14"
  khronos-egl = { version = "6", features = ["dynamic"] }
  ```

  Bring-up code (~250 LOC):
  ① open Window via tizen-window
  ② build EglWindow via tizen-egl
  ③ load libEGL.so via `khronos-egl::DynamicInstance::load_required()`
  ④ create EGL display, choose config, create context (try
    GLES 3.0 → 2.0 fallback)
  ⑤ create EGL surface from EglWindow
  ⑥ make context current
  ⑦ build `glow::Context::from_loader_function(|name|
    egl.get_proc_address(name).unwrap_or(ptr::null()))`
  ⑧ build `egui::Context` + `egui_glow::Painter`
  ⑨ render loop: on each `Redraw` event, run egui (animated
    content: fps counter, animated gradient, the egui demo Window),
    paint, eglSwapBuffers, request_redraw

- [ ] **3.2 Verify on .234.** Cross-build for armv7l, push, run for
      30 s, eyeball the TV. Capture WAYLAND_DEBUG trace if anything
      misbehaves.

## Out of scope (post-MVP)

Once the egui demo is up, these are the natural next steps:

- Full keyboard input via xkbcommon: keysyms, UTF-8 text-input,
  composed character entry. v1 ships raw evdev keycodes only.
- `wl_touch`: target has no touchscreen; same pattern as pointer
  when we want it.
- CPU backend: `Window::with_cpu_buffer` + egui→`tiny-skia`
  rasterizer. For when GPU is overkill or unavailable.
- `tizen-policy` features: window type, brightness, screen mode
  (the rest of what `efl_util_window.c` covers).
- Multi-window / subsurfaces / popups.
