//! Raw FFI for `libwayland-egl.so.1` — resolved at runtime via
//! `libloading::Library::new("libwayland-egl.so.1")` because the
//! Tizen Studio rootstrap doesn't ship a build-time symlink for this
//! library, although every Tizen device has the `.so.1` present.
//!
//! Wraps the three symbols a Wayland GLES client needs:
//!
//! * `wl_egl_window_create` / `_destroy` / `_resize`
//!
//! Same dlopen pattern as `tizen-tbm-sys::wayland_tbm`.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use core::ffi::{c_int, c_void};
use std::sync::OnceLock;

#[repr(C)]
pub struct wl_egl_window {
    _private: [u8; 0],
}

/// Function-pointer table loaded lazily on first use.
struct Api {
    create: unsafe extern "C" fn(
        surface: *mut c_void,
        width: c_int,
        height: c_int,
    ) -> *mut wl_egl_window,
    destroy: unsafe extern "C" fn(egl_window: *mut wl_egl_window),
    resize: unsafe extern "C" fn(
        egl_window: *mut wl_egl_window,
        width: c_int,
        height: c_int,
        dx: c_int,
        dy: c_int,
    ),
    /// Keep the loaded library alive for the lifetime of the process —
    /// dropping it would unload the .so and dangle the fn pointers.
    _lib: libloading::Library,
}

/// Errors from the dlopen path.
#[derive(Debug)]
pub enum LoadError {
    /// `dlopen("libwayland-egl.so.1")` failed.
    LibraryOpen(String),
    /// A required symbol was missing from the loaded `.so`.
    MissingSymbol(&'static str, String),
}

impl core::fmt::Display for LoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LibraryOpen(e) => write!(f, "failed to dlopen libwayland-egl.so.1: {e}"),
            Self::MissingSymbol(name, e) => {
                write!(f, "libwayland-egl.so.1 missing symbol `{name}`: {e}")
            }
        }
    }
}

impl std::error::Error for LoadError {}

static API: OnceLock<Result<Api, LoadError>> = OnceLock::new();

fn api() -> Result<&'static Api, &'static LoadError> {
    API.get_or_init(load).as_ref()
}

fn load() -> Result<Api, LoadError> {
    // SONAME of the .so on every Tizen device.
    let lib = unsafe { libloading::Library::new("libwayland-egl.so.1") }
        .map_err(|e| LoadError::LibraryOpen(e.to_string()))?;

    macro_rules! sym {
        ($name:literal) => {{
            let sym: libloading::Symbol<_> = unsafe { lib.get($name.as_bytes()) }
                .map_err(|e| LoadError::MissingSymbol($name, e.to_string()))?;
            *sym
        }};
    }

    Ok(Api {
        create: sym!("wl_egl_window_create"),
        destroy: sym!("wl_egl_window_destroy"),
        resize: sym!("wl_egl_window_resize"),
        _lib: lib,
    })
}

/// Returns `true` if `libwayland-egl.so.1` is loadable and all three
/// symbols resolved. Useful for failing fast at startup.
pub fn is_available() -> bool {
    api().is_ok()
}

/// `wl_egl_window_create(surface, width, height)`.
///
/// # Safety
///
/// `surface` must be a live `wl_surface *` for the duration of the
/// returned `wl_egl_window`. Caller must free with [`destroy`].
pub unsafe fn create(
    surface: *mut c_void,
    width: c_int,
    height: c_int,
) -> Result<*mut wl_egl_window, &'static LoadError> {
    let api = api()?;
    Ok((api.create)(surface, width, height))
}

/// `wl_egl_window_destroy(egl_window)`.
///
/// # Safety
///
/// `egl_window` must come from a prior successful [`create`].
pub unsafe fn destroy(egl_window: *mut wl_egl_window) {
    if let Ok(api) = api() {
        (api.destroy)(egl_window);
    }
}

/// `wl_egl_window_resize(egl_window, w, h, dx, dy)`.
///
/// # Safety
///
/// `egl_window` must come from a prior successful [`create`].
pub unsafe fn resize(
    egl_window: *mut wl_egl_window,
    width: c_int,
    height: c_int,
    dx: c_int,
    dy: c_int,
) -> Result<(), &'static LoadError> {
    let api = api()?;
    (api.resize)(egl_window, width, height, dx, dy);
    Ok(())
}
