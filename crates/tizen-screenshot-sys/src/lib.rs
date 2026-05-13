//! Raw client bindings for the Tizen `tizen_screenshooter` Wayland protocol
//! plus the FFI surface efl_util uses against `libtbm.so` and
//! `libwayland-tbm-client.so`.
//!
//! Like `tizen-input-sys` this crate is **not** `#![no_std]` — the
//! `wayland-scanner` output uses `String`/`Vec`. The hand-rolled C FFI half
//! is plain `extern "C"` though, and the safe wrapper in `tizen-screenshot`
//! still gates everything on `cfg(tizen)`.
//!
//! ## Library availability
//!
//! * `libtbm.so.1` ships in the Tizen Studio rootstrap, so it is linked
//!   directly via `#[cfg_attr(tizen, link(name = "tbm", kind = "dylib"))]`.
//! * `libwayland-tbm-client.so.0` is **not** in the rootstrap, so the
//!   handful of functions we need are resolved at runtime via
//!   `libloading::Library::new("libwayland-tbm-client.so.0")`. The .so is
//!   present on every Tizen device.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::all)]

pub mod protocol;
pub mod tbm;
pub mod wayland_tbm;
