//! Raw client bindings for the Tizen `tizen_screenshooter` Wayland
//! protocol.
//!
//! The TBM half of what efl_util_screenshot.c uses now lives in the
//! shared [`tizen-tbm-sys`] crate and is re-exported from here for
//! source compatibility.
//!
//! ## Library availability
//!
//! * `libwayland-client.so.0` is dlopen'd by `wayland-backend`.
//! * `libtbm.so.1` and `libwayland-tbm-client.so.0` are resolved as
//!   documented in [`tizen-tbm-sys`].
//!
//! [`tizen-tbm-sys`]: https://crates.io/crates/tizen-tbm-sys

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::all)]

pub mod protocol;

// Re-exports for downstream source compatibility: old call sites do
// `use tizen_screenshot_sys::{tbm, wayland_tbm};`. New code should
// import directly from `tizen-tbm-sys`.
pub use tizen_tbm_sys::tbm;
pub use tizen_tbm_sys::wayland_tbm;
