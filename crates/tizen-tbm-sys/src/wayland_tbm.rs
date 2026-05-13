//! Raw FFI for `libwayland-tbm-client.so.0`.
//!
//! Resolved at runtime via `libloading::Library::new("libwayland-tbm-client.so.0")`
//! because the Tizen Studio rootstrap doesn't ship this `.so` (the device
//! does — every Wayland-capable Tizen system has it). Wraps the subset of
//! `<wayland-tbm-client.h>` that `efl_util_screenshot.c` uses:
//!
//! * `wayland_tbm_client_init` / `_deinit`
//! * `wayland_tbm_client_create_buffer` / `_destroy_buffer`
//! * `wayland_tbm_client_set_event_queue`  — route protocol traffic
//!   through our private `wl_event_queue` (matches the C pattern).
//! * `wayland_tbm_client_get_bufmgr`        — used by efl_util only for
//!   error-path validation; we expose it for parity.

use core::ffi::c_void;
use std::sync::OnceLock;

use crate::tbm::tbm_surface_h;

/// `wayland_tbm_client *` from `<wayland-tbm-client.h>`. Opaque.
#[repr(C)]
pub struct wayland_tbm_client {
    _opaque: [u8; 0],
}

/// Function-pointer table loaded lazily on first use. Each entry is the
/// `extern "C"` function as declared in `wayland-tbm-client.h`.
struct Api {
    init: unsafe extern "C" fn(display: *mut c_void) -> *mut wayland_tbm_client,
    deinit: unsafe extern "C" fn(client: *mut wayland_tbm_client),
    create_buffer: unsafe extern "C" fn(
        client: *mut wayland_tbm_client,
        surface: tbm_surface_h,
    ) -> *mut c_void,
    destroy_buffer: unsafe extern "C" fn(client: *mut wayland_tbm_client, buffer: *mut c_void),
    set_event_queue:
        unsafe extern "C" fn(client: *mut wayland_tbm_client, queue: *mut c_void) -> i32,
    get_bufmgr: unsafe extern "C" fn(client: *mut wayland_tbm_client) -> *mut c_void,
    // Keep the loaded library alive for the lifetime of the process —
    // dropping it would unload the .so and dangle the fn pointers.
    _lib: libloading::Library,
}

/// Errors from the dlopen path.
#[derive(Debug)]
pub enum LoadError {
    /// `dlopen("libwayland-tbm-client.so.0")` failed.
    LibraryOpen(String),
    /// A required symbol was missing from the loaded `.so`.
    MissingSymbol(&'static str, String),
}

impl core::fmt::Display for LoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LibraryOpen(e) => {
                write!(f, "failed to dlopen libwayland-tbm-client.so.0: {e}")
            }
            Self::MissingSymbol(name, e) => {
                write!(f, "libwayland-tbm-client.so.0 missing symbol `{name}`: {e}")
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
    // SONAME of the .so on every Tizen device. Don't try `.so` (no
    // version suffix) — that's the dev symlink, only in -devel packages.
    let lib = unsafe { libloading::Library::new("libwayland-tbm-client.so.0") }
        .map_err(|e| LoadError::LibraryOpen(e.to_string()))?;

    macro_rules! sym {
        ($name:literal) => {{
            let sym: libloading::Symbol<_> = unsafe { lib.get($name.as_bytes()) }
                .map_err(|e| LoadError::MissingSymbol($name, e.to_string()))?;
            *sym
        }};
    }

    Ok(Api {
        init: sym!("wayland_tbm_client_init"),
        deinit: sym!("wayland_tbm_client_deinit"),
        create_buffer: sym!("wayland_tbm_client_create_buffer"),
        destroy_buffer: sym!("wayland_tbm_client_destroy_buffer"),
        set_event_queue: sym!("wayland_tbm_client_set_event_queue"),
        get_bufmgr: sym!("wayland_tbm_client_get_bufmgr"),
        _lib: lib,
    })
}

/// Returns `true` if the `.so` could be loaded and all needed symbols
/// resolved. Useful to fail fast at startup.
pub fn is_available() -> bool {
    api().is_ok()
}

/// `wayland_tbm_client_init(display)` — initialise the TBM client with
/// the caller's `wl_display *`.
///
/// # Safety
///
/// `display` must be a valid live `*mut wl_display` for the duration of
/// the returned client. Use [`deinit`] to release.
pub unsafe fn init(display: *mut c_void) -> Result<*mut wayland_tbm_client, &'static LoadError> {
    let api = api()?;
    Ok((api.init)(display))
}

/// `wayland_tbm_client_deinit(client)` — release a TBM client.
///
/// # Safety
///
/// `client` must come from a prior successful [`init`].
pub unsafe fn deinit(client: *mut wayland_tbm_client) {
    if let Ok(api) = api() {
        (api.deinit)(client);
    }
}

/// `wayland_tbm_client_create_buffer(client, surface)` — wrap a
/// [`tbm_surface_h`](crate::tbm::tbm_surface_h) as a `wl_buffer *` for
/// use with Wayland requests like `tizen_screenshooter.shoot`.
///
/// # Safety
///
/// Both `client` and `surface` must be valid for the duration of the
/// returned buffer. Free with [`destroy_buffer`].
pub unsafe fn create_buffer(
    client: *mut wayland_tbm_client,
    surface: tbm_surface_h,
) -> Result<*mut c_void, &'static LoadError> {
    let api = api()?;
    Ok((api.create_buffer)(client, surface))
}

/// `wayland_tbm_client_destroy_buffer(client, buffer)` — release a
/// wl_buffer created via [`create_buffer`].
///
/// # Safety
///
/// `client` must be the same one passed to the prior `create_buffer`;
/// `buffer` must be that call's return value.
pub unsafe fn destroy_buffer(client: *mut wayland_tbm_client, buffer: *mut c_void) {
    if let Ok(api) = api() {
        (api.destroy_buffer)(client, buffer);
    }
}

/// `wayland_tbm_client_set_event_queue(client, queue)` — route this TBM
/// client's `wl_tbm` protocol traffic through `queue` instead of the
/// display's default queue. Lets us dispatch only the screenshot-related
/// events synchronously while leaving the rest of the connection alone.
///
/// # Safety
///
/// `client` must be a valid TBM client; `queue` must be a valid
/// `wl_event_queue *` for the same display, alive for the duration of
/// this binding.
pub unsafe fn set_event_queue(
    client: *mut wayland_tbm_client,
    queue: *mut c_void,
) -> Result<i32, &'static LoadError> {
    let api = api()?;
    Ok((api.set_event_queue)(client, queue))
}

/// `wayland_tbm_client_get_bufmgr(client)` — fetch the underlying
/// `tbm_bufmgr *`. efl_util uses this only for validation.
///
/// # Safety
///
/// `client` must be a valid TBM client.
pub unsafe fn get_bufmgr(
    client: *mut wayland_tbm_client,
) -> Result<*mut c_void, &'static LoadError> {
    let api = api()?;
    Ok((api.get_bufmgr)(client))
}
