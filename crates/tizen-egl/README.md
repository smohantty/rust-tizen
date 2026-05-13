# tizen-egl

Safe wrapper for `libwayland-egl` on Tizen. Builds a `wl_egl_window` —
the handle `eglCreateWindowSurface(..., NativeWindowType, ...)` wants —
on top of any windowing crate that exposes a Wayland `raw-window-handle`.

This crate is **not coupled to `tizen-window`**: `EglWindow::new` takes
`impl HasWindowHandle`, so any future windowing crate (or test mock)
that emits a `RawWindowHandle::Wayland` works the same.

```rust,no_run
use tizen_window::{Display, WindowBuilder};
use tizen_egl::EglWindow;

let display = Display::connect()?;
let window = WindowBuilder::new().size(1920, 1080).build(&display)?;

let egl = EglWindow::new(&window, 1920, 1080)?;
// hand egl.as_ptr() to eglCreateWindowSurface(...).
# Ok::<_, Box<dyn std::error::Error>>(())
```

The lifetime parameter on `EglWindow<'w>` keeps the windowing handle
alive for as long as the EGL window holds a pointer into it — the
compiler enforces ordering instead of a runtime contract.

## Cargo feature

`tizen-egl` is exposed via the umbrella crate as the `egl` feature:

```toml
tizen = { version = "0.1", features = ["window", "egl"] }
```
