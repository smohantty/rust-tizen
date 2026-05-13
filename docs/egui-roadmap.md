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
│   1.3 ☑ Repaint on configure (compositor resize)             │
│   1.4 ☑ Display::run event-loop helper                       │
│   1.5 ☑ wl_pointer wiring (enter/leave/motion/button/wheel)  │
│   1.6 ☑ wl_keyboard wiring (raw keycodes; xkb deferred)      │
├──────────────────────────────────────────────────────────────┤
│ Phase 2 — `tizen-egl` crate + EGL probe                      │
│   2.1 ☑ tizen-egl-sys (wl_egl_window FFI)                    │
│   2.2 ☑ tizen-egl safe wrapper                               │
│   2.3 ☑ Umbrella `egl` feature                               │
│   2.4 ☑ examples/hello-egl-probe — GO: GLES 3.2 on Mali-G51  │
├──────────────────────────────────────────────────────────────┤
│ Phase 3 — egui hello world                                   │
│   3.1 ☑ examples/hello-egui-gpu — egui_glow + tizen-egl      │
│   3.2 ☑ Verified on .234 — steady 60 FPS on Mali-G51         │
└──────────────────────────────────────────────────────────────┘

**🎉 Goal hit:** egui renders on Tizen at 60 FPS via the rust-tizen
stack. See "Phase 3" below for the verification transcript.
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

- [x] **1.3 Repaint on configure.** `Dispatch<ZxdgSurfaceV6>` pushes
      `Event::RedrawRequested` on the first configure; `Dispatch<ZxdgToplevelV6>`
      pairs `Event::Resized { w, h }` with `RedrawRequested` whenever the
      compositor delivers a new non-zero size. `examples/hello-window`
      drains events and re-fills on `Resized`, so the buffer now matches
      the compositor's chosen size instead of staying pinned at 640×480.

- [x] **1.4 Display::run event-loop helper.** `Display::run(&mut self,
      window, callback)` blocks dispatching events and delivers each
      `Event` to the user closure with `&mut Window` (so the closure
      can `fill_solid`, `request_redraw`, etc.). Returns when the
      compositor sends close. Pending events queued before the first
      dispatch (e.g. the initial `RedrawRequested` from the first
      configure) are drained *before* the first `blocking_dispatch`,
      so the closure always sees the first frame.

      The `Event` enum itself landed in Phase 1.2 — variant names and
      field shapes mirror `winit::event::WindowEvent`. Pointer and
      keyboard variants get wired in 1.5 / 1.6.

- [x] **1.5 wl_pointer wiring.** `Dispatch<WlSeat>` listens for the
      `capabilities` event and calls `seat.get_pointer(...)` on the
      first `Pointer` cap; the resulting `WlPointer` lives in
      `WindowState::pointer`. `Dispatch<WlPointer>` maps:

  | wl_pointer event | crate::Event |
  |---|---|
  | `enter { surface_x, surface_y }` | `CursorEntered { x, y }` |
  | `leave` | `CursorLeft` |
  | `motion { surface_x, surface_y }` | `CursorMoved { x, y }` |
  | `button { button, state }` | `MouseInput { button, pressed }` |
  | `axis { axis, value }` | `MouseWheel { dx, dy }` |

  Evdev `BTN_*` codes (0x110…0x114) are translated to the
  winit-shaped `MouseButton` enum; everything else surfaces as
  `MouseButton::Other(code as u16)`. `hello-window` logs every
  pointer event for on-device verification.

- [x] **1.6 wl_keyboard wiring (raw keycodes).** `Dispatch<WlSeat>`
      also binds `wl_keyboard` on the `Keyboard` capability;
      `WindowState::keyboard` holds it. `Dispatch<WlKeyboard>` maps:

  | wl_keyboard event | crate::Event |
  |---|---|
  | `enter { surface, keys }` | `Focused(true)` |
  | `leave { surface }` | `Focused(false)` |
  | `key { key, state }` | `KeyboardInput { keycode: key, pressed, modifiers: empty }` |

  `keycode` is the raw evdev value (`KEY_ESC=1`, `KEY_ENTER=28`,
  `KEY_LEFT/RIGHT/UP/DOWN=105/106/103/108`, …). The `keymap` event
  is consumed-and-dropped, which closes the keymap fd — we don't
  parse it yet. The `modifiers` event is silently ignored because
  the wire format ships XKB mod *indices* whose meaning depends on
  the keymap; without xkbcommon we cannot honestly populate
  `ModifiersState`, so every `KeyboardInput` carries
  `ModifiersState::empty()`. Full xkbcommon + UTF-8 text input is
  post-MVP. `hello-window` logs every key event so the on-device
  transcript can show the remote talking.

**Phase 1 is now complete — all of Phase 2 can begin.**

## Phase 2 — `tizen-egl` + capability probe

The single new platform crate. Wraps `libwayland-egl.so.1` (in the
rootstrap, no dlopen needed) and exposes an `EglWindow` that
`khronos-egl` can use to create a window surface.

- [x] **2.1 `tizen-egl-sys`.** New crate at `crates/tizen-egl-sys`
      that resolves `wl_egl_window_create / _destroy / _resize` via
      `libloading::Library::new("libwayland-egl.so.1")` on first use.
      The Tizen 10 rootstrap turned out to lack a build-time symlink
      for `libwayland-egl` even though every device ships the `.so.1`,
      so we use the same dlopen pattern as
      `tizen-tbm-sys::wayland_tbm`. Public API: `is_available()` +
      `create` / `destroy` / `resize` returning
      `Result<_, &'static LoadError>`. Registered in
      `[workspace.dependencies]`.

- [x] **2.2 `tizen-egl`.** Safe wrapper. Public:

  ```rust
  pub struct EglWindow<'w> { /* … */ }
  impl<'w> EglWindow<'w> {
      pub fn new<W: HasWindowHandle>(window: &'w W, w: u32, h: u32) -> Result<Self>;
      pub fn as_ptr(&self) -> *mut c_void;            // hand to eglCreateWindowSurface
      pub fn resize(&self, w: u32, h: u32, dx: i32, dy: i32);
  }
  // Drop frees the wl_egl_window.
  ```

  The actual API is generic over [`raw_window_handle::HasWindowHandle`]
  rather than the roadmap's original `&tizen_window::Window` — this
  keeps `tizen-egl` free of any inter-platform-crate dependency
  (matching the ecosystem convention winit/glutin use). The
  lifetime parameter ties the EGL window to the windowing handle's
  borrow, so the compiler enforces "wl_surface outlives wl_egl_window"
  instead of a runtime contract.

- [x] **2.3 Umbrella `egl` feature.** `tizen/Cargo.toml` exposes
      `egl = ["window", "dep:tizen-egl"]`; `tizen/src/lib.rs`
      re-exports the crate as `tizen::egl` behind the same feature.
      `cargo check -p tizen --features egl` clean.

- [x] **2.4 `hello-egl-probe`.** Standalone example at
      `examples/hello-egl-probe` that opens a Window, builds an
      `EglWindow`, dynamically loads `libEGL.so` via
      `khronos-egl`, asks for a GLES 3 context (falling back to 2),
      makes the context current, and prints `eglQueryString` +
      `glGetString` via `glow`.

      **On-device verdict (.234, Tizen 10/TV armv7l):** GO.

      ```text
      EGL_VERSION       1.5
      EGL_VENDOR        ARM
      EGL_CLIENT_APIS   OpenGL_ES
      GL_VENDOR         ARM
      GL_RENDERER       Mali-G51
      GL_VERSION        OpenGL ES 3.2 v1.r48p0-01eac0…
      GL_GLSL           OpenGL ES GLSL ES 3.20
      ```

      GLES 3.2 + GLSL 3.20 → `egui_glow` is on for Phase 3.

**Phase 2 is complete — Phase 3 (egui hello world) can begin.**

## Phase 3 — egui hello world

The payoff. `examples/hello-egui-gpu/` opens a window, paints an
animated egui UI via the GPU.

- [x] **3.1 `hello-egui-gpu` crate.** `examples/hello-egui-gpu`,
      ~190 LOC. Pulls `tizen = { features = ["window", "egl"] }`,
      `egui = "0.29"`, `egui_glow = "0.29"`, `glow = "0.14"`,
      `khronos-egl = "6"` with the `dynamic` feature. The bring-up
      follows the roadmap recipe exactly:

      ① open `Window` via `tizen-window`
      ② build `EglWindow` via `tizen-egl` (note: `unsafe` now;
        see the EglWindow lifetime refactor in commit 16dddc8)
      ③ load `libEGL.so` via `khronos-egl::DynamicInstance<EGL1_4>::load_required()`
      ④ get EGL display from the Wayland `wl_display *`, init,
        `bind_api(OPENGL_ES_API)`, choose 8/8/8/0 config
      ⑤ create context — GLES 3 first, fall back to 2
      ⑥ `create_window_surface(... egl_window.as_ptr() ...)` +
        `make_current`
      ⑦ build `glow::Context::from_loader_function(get_proc_address)`
      ⑧ build `egui::Context` + `egui_glow::Painter`
      ⑨ `Display::run` closure handles `Resized → egl_window.resize`
        and `RedrawRequested → run egui → paint → swap_buffers →
        request_redraw`. UI: a `CentralPanel` with title + frame
        counter + elapsed seconds, plus an animated floating
        `Window` with a sine-wave `ProgressBar`. Clear colour also
        animates in case egui ever stops painting.

- [x] **3.2 Verify on .234.** Cross-build for armv7l, push via
      `rsdb agent transfer.push`, run on the TV. **Steady 60 FPS,
      visually confirmed** — both windows render (full-screen
      `CentralPanel` + floating "animation" popup with progress
      bar). 25-second transcript:

      ```text
      hello-egui-gpu: configured at 1920x1080
      hello-egui-gpu: EGL 1.5
      hello-egui-gpu: GLES 3 context
      hello-egui-gpu: GL_VERSION = OpenGL ES 3.2 v1.r48p0-01eac0…
      hello-egui-gpu: frame=44   fps≈43.6      (first second, init overhead)
      hello-egui-gpu: frame=105  fps≈60.1
      hello-egui-gpu: frame=165  fps≈60.0
      …
      hello-egui-gpu: frame=1428 fps≈60.0      (25 s in, still vsync'd)
      ```

      1428 frames / 25 s ≈ 57 FPS average (including the slower
      first second); steady-state is 60 FPS exactly, vsync-locked
      against the compositor's frame callbacks.

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
