//! Raw FFI bindings for `libwayland-egl.so.1`.
//!
//! `wl_egl_window` is the Wayland-side glue between a `wl_surface` and
//! an EGL window surface — `eglCreateWindowSurface(..., wl_egl_window,
//! ...)` is the GLES bring-up pattern on every Wayland desktop. The
//! library ships in the Tizen rootstrap, so we link directly via
//! `#[link]` (no `dlopen` shim needed).
//!
//! These are extern declarations only — see the `tizen-egl` crate for
//! the safe wrapper.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![no_std]

use core::ffi::{c_int, c_void};

#[repr(C)]
pub struct wl_egl_window {
    _private: [u8; 0],
}

#[cfg_attr(tizen, link(name = "wayland-egl", kind = "dylib"))]
extern "C" {
    pub fn wl_egl_window_create(
        surface: *mut c_void,
        width: c_int,
        height: c_int,
    ) -> *mut wl_egl_window;

    pub fn wl_egl_window_destroy(egl_window: *mut wl_egl_window);

    pub fn wl_egl_window_resize(
        egl_window: *mut wl_egl_window,
        width: c_int,
        height: c_int,
        dx: c_int,
        dy: c_int,
    );
}
