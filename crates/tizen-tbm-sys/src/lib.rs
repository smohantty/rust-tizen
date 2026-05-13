//! Raw FFI bindings for the Tizen Buffer Manager (TBM) and the
//! `libwayland-tbm-client` shim used by Tizen Wayland clients to wrap
//! TBM buffers as `wl_buffer` objects.
//!
//! This crate is the shared "tbm FFI" base used by multiple higher-level
//! Tizen crates (`tizen-screenshot`, `tizen-window`, future video
//! shell, …). It contains only `extern "C"` declarations + opaque
//! pointer types — no Wayland-protocol Rust bindings.
//!
//! ## Library availability
//!
//! * `libtbm.so.1` is in the Tizen Studio rootstrap, so the `tbm`
//!   module uses direct `#[link]`.
//! * `libwayland-tbm-client.so.0` is **not** in the rootstrap. The
//!   `wayland_tbm` module uses `libloading::Library::new(...)` to
//!   resolve it at first use; the `.so` is present on every Tizen
//!   device.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::all)]

pub mod tbm;
pub mod wayland_tbm;
