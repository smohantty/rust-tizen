# tizen-egl-sys

Raw FFI bindings for `libwayland-egl.so.1` — the `wl_egl_window` helper
Wayland clients use to create an EGL window surface on top of a
`wl_surface`.

The Tizen Studio rootstrap doesn't ship a build-time symlink for
`libwayland-egl.so`, so this crate resolves the three symbols via
`libloading::Library::new("libwayland-egl.so.1")` at first use. The
`.so.1` is present on every Tizen device. Same pattern as
`tizen-tbm-sys::wayland_tbm`.

## Symbols

* `create(surface, width, height) -> Result<*mut wl_egl_window, &LoadError>`
* `destroy(egl_window)`
* `resize(egl_window, width, height, dx, dy) -> Result<(), &LoadError>`

Use the `tizen-egl` crate for the safe wrapper.
