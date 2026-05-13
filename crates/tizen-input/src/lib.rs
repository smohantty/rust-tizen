//! Inject key/touch/pointer events into a Tizen Wayland compositor via the
//! `tizen_input_device_manager` global protocol.
//!
//! ```no_run
//! use tizen_input::{DeviceType, InputGenerator, KeyState};
//!
//! let mut gen = InputGenerator::builder()
//!     .name("hello-input")
//!     .device(DeviceType::KEYBOARD)
//!     .open()?;
//!
//! gen.key("XF86Back", KeyState::Pressed)?;
//! gen.key("XF86Back", KeyState::Released)?;
//! # Ok::<(), tizen_input::Error>(())
//! ```
//!
//! ## Backend
//!
//! Talks Wayland directly via [`wayland-client`] using the `client_system` +
//! `dlopen` backend, so the crate adds no build-time link dependency and at
//! runtime only resolves `libwayland-client.so.0` (already present on every
//! Tizen device).
//!
//! ## Off-target behaviour
//!
//! Following the project convention, calls compile and link on host. With
//! `cfg(not(tizen))` the public API still works:
//!
//! * [`InputGenerator::open`] either talks to the host compositor (if
//!   `$WAYLAND_DISPLAY` is set and reachable) **or** returns
//!   [`Error::Unsupported`]. It never panics or aborts.
//! * Successful calls on host write a one-line trace to stderr so dev
//!   loops without a device still see the action.

#![warn(missing_docs)]

mod error;
mod generator;

pub use error::{Error, Result};
pub use generator::{
    DeviceType, InputGenerator, InputGeneratorBuilder, KeyState, PointerButton, PointerPhase,
    TouchPhase,
};
