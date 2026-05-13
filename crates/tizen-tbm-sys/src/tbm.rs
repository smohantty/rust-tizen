//! Raw FFI for `libtbm.so.1` — the Tizen Buffer Manager.
//!
//! Mirrors the subset of `<tbm_surface.h>` / `<tbm_type.h>` that the
//! screenshot path needs: surface create/destroy, map/unmap for pixel
//! readback, and one fourcc format constant. `tbm_surface_h` and the
//! `bufmgr` are opaque from Rust's perspective.

use core::ffi::{c_int, c_uint, c_void};

/// `tbm_surface_h` from `<tbm_type.h>`. Opaque — never dereferenced by Rust.
pub type tbm_surface_h = *mut c_void;

/// `tbm_format` from `<tbm_type.h>` — a fourcc-style u32.
pub type tbm_format = u32;

/// Const-fn equivalent of `__tbm_fourcc_code(a, b, c, d)`.
pub const fn tbm_fourcc(a: u8, b: u8, c: u8, d: u8) -> tbm_format {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

/// `TBM_FORMAT_XRGB8888` = fourcc('X','R','2','4') = 0x34325258.
/// 32-bit packed, little-endian, alpha ignored. The format efl_util
/// requests for screenshots and the one we'll default to.
pub const TBM_FORMAT_XRGB8888: tbm_format = tbm_fourcc(b'X', b'R', b'2', b'4');

/// `TBM_SURF_PLANE_MAX` from `<tbm_surface.h>`.
pub const TBM_SURF_PLANE_MAX: usize = 4;

/// `TBM_SURF_OPTION_READ` — bitflag passed to `tbm_surface_map`.
pub const TBM_SURF_OPTION_READ: c_int = 1 << 0;
/// `TBM_SURF_OPTION_WRITE` — bitflag passed to `tbm_surface_map`.
pub const TBM_SURF_OPTION_WRITE: c_int = 1 << 1;

/// `tbm_surface_plane_s` from `<tbm_surface.h>`. Plane-N pixel data lives
/// at `ptr[offset..offset+size]`; rows are `stride` bytes apart.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct tbm_surface_plane_s {
    pub ptr: *mut u8,
    pub size: u32,
    pub offset: u32,
    pub stride: u32,
    pub reserved1: *mut c_void,
    pub reserved2: *mut c_void,
    pub reserved3: *mut c_void,
}

/// `tbm_surface_info_s` from `<tbm_surface.h>`. Filled in by `tbm_surface_map`
/// or `tbm_surface_get_info` to describe the buffer's pixel layout.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct tbm_surface_info_s {
    pub width: u32,
    pub height: u32,
    pub format: tbm_format,
    pub bpp: u32,
    pub size: u32,
    pub num_planes: u32,
    pub planes: [tbm_surface_plane_s; TBM_SURF_PLANE_MAX],
    pub reserved4: *mut c_void,
    pub reserved5: *mut c_void,
    pub reserved6: *mut c_void,
}

impl Default for tbm_surface_info_s {
    fn default() -> Self {
        // SAFETY: `tbm_surface_info_s` is plain-old-data with `*mut c_void`
        // and integer fields; the all-zero representation is a valid value
        // (null pointers, zero sizes).
        unsafe { core::mem::zeroed() }
    }
}

#[cfg_attr(tizen, link(name = "tbm", kind = "dylib"))]
extern "C" {
    /// Allocate a buffer of the requested size and format.
    pub fn tbm_surface_create(width: c_int, height: c_int, format: tbm_format) -> tbm_surface_h;

    /// Free the buffer. Returns `TBM_SURFACE_ERROR_NONE` (0) on success.
    pub fn tbm_surface_destroy(surface: tbm_surface_h) -> c_int;

    /// Map the buffer's pixel memory for `opt` (READ / WRITE bitmask).
    /// On success `*info` is filled with stride/plane info and
    /// `info.planes[0].ptr` points at the start of the pixel data.
    pub fn tbm_surface_map(
        surface: tbm_surface_h,
        opt: c_int,
        info: *mut tbm_surface_info_s,
    ) -> c_int;

    /// Release the mapping established by `tbm_surface_map`.
    pub fn tbm_surface_unmap(surface: tbm_surface_h) -> c_int;

    /// Get the buffer's metadata without mapping its memory.
    pub fn tbm_surface_get_info(surface: tbm_surface_h, info: *mut tbm_surface_info_s) -> c_int;

    /// Convenience accessors (`int`-returning, error code is negative).
    pub fn tbm_surface_get_width(surface: tbm_surface_h) -> c_int;
    pub fn tbm_surface_get_height(surface: tbm_surface_h) -> c_int;
    pub fn tbm_surface_get_format(surface: tbm_surface_h) -> tbm_format;
}

/// Convenience: ignore `c_int` return value (Tizen FFI is consistent in
/// returning `TBM_SURFACE_ERROR_NONE = 0` on success).
#[allow(dead_code)]
#[inline]
pub fn ok(rc: c_int) -> bool {
    rc == 0
}

// Tag this in so the `c_uint` import is exercised when callers want to
// reach the raw `tbm_format` codes by name; lifts a clippy warning on
// crates that re-export `protocol::tbm`.
#[doc(hidden)]
pub type _TbmFormat = c_uint;
