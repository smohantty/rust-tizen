# tizen-egl-sys

Raw `extern "C"` bindings for `libwayland-egl.so.1` — the `wl_egl_window`
helper Wayland clients use to create an EGL window surface on top of a
`wl_surface`.

This is a `-sys` crate: `#![no_std]`, no dependencies, no safety. Use
the `tizen-egl` crate for the safe wrapper.

## Symbols

* `wl_egl_window_create(surface, width, height) -> *mut wl_egl_window`
* `wl_egl_window_destroy(egl_window)`
* `wl_egl_window_resize(egl_window, width, height, dx, dy)`

## Linking

The library is in the Tizen Studio rootstrap, so we link directly:

```rust
#[cfg_attr(tizen, link(name = "wayland-egl", kind = "dylib"))]
extern "C" { … }
```

`cargo-tizen` sets `--cfg tizen` during the cross-build; plain
`cargo check` on the host compiles the declarations without producing
a link directive.
