# tizen-window

Native Tizen Wayland window — pure Rust, **no EFL/Ecore dependency**.

```rust
use tizen_window::{Display, WindowBuilder};

let mut display = Display::connect()?;
let mut window = WindowBuilder::new()
    .title("hello-window")
    .app_id("rust.tizen.hello")
    .size(640, 480)
    .build(&display)?;

while !window.should_close() {
    display.dispatch_pending(&mut window)?;
}
```

## What this is

A direct Rust re-implementation of the window-creation slice of
[`tizen-core-wayland`](https://git.tizen.org/cgit/platform/core/uifw/tizen-core-wayland)
— the EFL-free Wayland client library that Samsung is shipping as
the replacement for `ecore_wl2`. We follow the exact same protocol
dance:

1. Connect, bind `wl_compositor` + `zxdg_shell_v6` + `wtz_shell` +
   `wl_seat`.
2. `wl_compositor.create_surface` → `wl_surface`.
3. `zxdg_shell_v6.get_xdg_surface` → `zxdg_surface_v6`.
4. `zxdg_surface_v6.get_toplevel` → `zxdg_toplevel_v6`; set
   title + app_id.
5. `wtz_shell.get_wtz_surface` → `wtz_surface`.
6. Initial commit → wait for `configure` → ack → paint.

## Why `zxdg_shell_v6` (not `xdg_wm_base`)?

Tizen 10/11 TV builds advertise `zxdg_shell_v6` v1 as the
window-management role. The modern stable `xdg_wm_base` (what
winit / sctk / wgpu expect) is **not** advertised. The
`tizen-core-wayland` source (`tizen_core_wl_surface.c:471`)
confirms — every Tizen-native window goes through v6. Using this
crate gets you on the same protocol path as a real Tizen app.

## Runtime deps

| Library | Resolution |
|---|---|
| `libwayland-client.so.0` | dlopen by `wayland-backend` |
| `libtbm.so.1` | `#[link]` under `cfg(tizen)` (in rootstrap) |
| `libwayland-tbm-client.so.0` | `libloading::Library::new(...)` at first use |

All three are on every Tizen device.

## Limitations (MVP slice)

* **Single window.** No popups / sub-surfaces yet.
* **CPU-rendered solid colour.** `Window::fill_solid(bgrx)` is the
  only paint operation. EGL / wgpu integration is a follow-up.
* **No keyboard / pointer / touch.** `wl_seat` is bound but events
  aren't wired through — use [`tizen-input`](../tizen-input/) for
  input injection in the meantime.
* **Auto-paint on configure only.** Frame callbacks / damage
  tracking left for later.
