//! Safe wrapper for `libwayland-egl` on Tizen.
//!
//! `EglWindow::new` is generic over [`raw_window_handle::HasWindowHandle`],
//! so this crate has **no dependency on `tizen-window`** — any
//! windowing crate that emits a Wayland `raw-window-handle` works.
//! See `README.md` for a full bring-up example using `tizen-window`.

#![warn(missing_docs)]

use core::marker::PhantomData;
use std::ffi::c_void;
use std::fmt;
use std::ptr::NonNull;

use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle};
use tizen_egl_sys::{wl_egl_window, LoadError};

/// Result alias for fallible `tizen-egl` operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Failure modes for [`EglWindow`].
#[derive(Debug)]
pub enum Error {
    /// The window did not yield a [`raw_window_handle::WindowHandle`].
    NoWindowHandle(HandleError),
    /// The window handle is not a Wayland surface — `tizen-egl` only
    /// supports the Wayland backend.
    NotWayland,
    /// Zero width or height was requested.
    InvalidSize,
    /// `wl_egl_window_create` returned NULL (out of memory).
    CreateFailed,
    /// `libwayland-egl.so.1` could not be loaded.
    LibraryLoad(&'static LoadError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoWindowHandle(e) => write!(f, "raw-window-handle: {e}"),
            Self::NotWayland => f.write_str("window handle is not Wayland"),
            Self::InvalidSize => f.write_str("width and height must be non-zero"),
            Self::CreateFailed => f.write_str("wl_egl_window_create returned NULL"),
            Self::LibraryLoad(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

/// A `wl_egl_window` — the Wayland-side glue between a `wl_surface`
/// and an EGL window surface.
///
/// The lifetime parameter ties the `EglWindow` to the window whose
/// surface it wraps. The underlying `wl_surface *` must outlive this
/// handle, which the borrow enforces.
pub struct EglWindow<'w> {
    ptr: NonNull<wl_egl_window>,
    _surface: PhantomData<&'w ()>,
}

impl<'w> EglWindow<'w> {
    /// Build a `wl_egl_window` from any window that exposes a
    /// [`raw_window_handle::WaylandWindowHandle`].
    pub fn new<W>(window: &'w W, width: u32, height: u32) -> Result<Self>
    where
        W: HasWindowHandle,
    {
        if width == 0 || height == 0 {
            return Err(Error::InvalidSize);
        }
        let handle = window.window_handle().map_err(Error::NoWindowHandle)?;
        let RawWindowHandle::Wayland(wl) = handle.as_raw() else {
            return Err(Error::NotWayland);
        };
        // SAFETY: `wl.surface` is a live `wl_surface *` for as long as
        // the borrow `&'w W` is held (the raw-window-handle contract).
        let raw = unsafe {
            tizen_egl_sys::create(wl.surface.as_ptr(), width as i32, height as i32)
                .map_err(Error::LibraryLoad)?
        };
        NonNull::new(raw)
            .map(|nn| Self {
                ptr: nn,
                _surface: PhantomData,
            })
            .ok_or(Error::CreateFailed)
    }

    /// Pointer suitable to hand to `eglCreateWindowSurface`. The EGL
    /// API expects `NativeWindowType` which on Wayland is
    /// `*mut wl_egl_window`.
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr() as *mut c_void
    }

    /// Resize the underlying `wl_egl_window`. `dx` / `dy` shift the
    /// origin (negative values move the visible area up/left); pass
    /// `0, 0` for a centred resize. Errors only if the underlying
    /// `libwayland-egl.so.1` cannot be re-resolved (it can't, since
    /// `new` already loaded it).
    pub fn resize(&self, width: u32, height: u32, dx: i32, dy: i32) -> Result<()> {
        unsafe {
            tizen_egl_sys::resize(self.ptr.as_ptr(), width as i32, height as i32, dx, dy)
                .map_err(Error::LibraryLoad)
        }
    }
}

impl Drop for EglWindow<'_> {
    fn drop(&mut self) {
        // SAFETY: `self.ptr` came from a successful `create`; we never
        // expose it for double-free. `destroy` silently no-ops if the
        // library somehow became unavailable.
        unsafe { tizen_egl_sys::destroy(self.ptr.as_ptr()) };
    }
}
