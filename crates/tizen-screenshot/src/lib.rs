//! Capture screenshots from a Tizen Wayland compositor.
//!
//! ```no_run
//! use tizen_screenshot::ScreenCapturer;
//!
//! let mut cap = ScreenCapturer::new()?;
//! let frame = cap.shoot()?;
//! std::fs::write("/tmp/shot.bgrx", frame.bytes()).ok();
//! # Ok::<(), tizen_screenshot::Error>(())
//! ```
//!
//! ## What this crate does (and doesn't)
//!
//! * Binds the Tizen `tizen_screenshooter` Wayland global.
//! * Allocates a TBM-backed `wl_buffer` via `libwayland-tbm-client`
//!   (resolved at runtime via dlopen — no rootstrap dependency).
//! * Asks the compositor to fill the buffer (`shoot` / `area_shoot`),
//!   waits for the matching `done` event.
//! * Maps the buffer and copies the pixels into a [`ScreenFrame`]
//!   that the caller owns. Frees the TBM buffer immediately after.
//!
//! What it does **not** do:
//!
//! * PNG / JPEG encoding — the frame is raw `XRGB8888` bytes; callers
//!   write or encode as they see fit.
//! * Continuous capture (screenmirror) — a separate crate / feature.
//! * Pick a specific output — uses the first `wl_output` the compositor
//!   advertises (matches efl_util behaviour).
//!
//! ## Off-target behaviour
//!
//! With `cfg(not(tizen))` `ScreenCapturer::new()` returns
//! [`Error::Unsupported`] immediately — there's no meaningful host
//! fallback, since the compositor and the TBM allocator are both
//! Tizen-specific. Off-target builds still compile and link.

#![warn(missing_docs)]

mod capturer;
mod error;

pub use capturer::{ScreenCapturer, ScreenFrame};
pub use error::{Error, Result};
