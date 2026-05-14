//! Native Tizen Wayland window — pure Rust, no EFL/Ecore dependency.
//!
//! ```no_run
//! use tizen_window::{Display, WindowBuilder};
//!
//! let mut display = Display::connect()?;
//! let mut window = WindowBuilder::new()
//!     .title("hello-window")
//!     .app_id("rust.tizen.hello")
//!     .size(640, 480)
//!     .build(&display)?;
//!
//! // Fill with a solid colour and run until the compositor closes us.
//! window.fill_solid(0xFF1E40AFu32);   // BGRX deep blue
//! while !window.should_close() {
//!     display.dispatch_pending(&mut window)?;
//! }
//! # Ok::<(), tizen_window::Error>(())
//! ```
//!
//! ## What this crate does
//!
//! Mirrors the minimum protocol dance `tizen-core-wayland`'s
//! `tizen_core_wl_create_window` performs:
//!
//! 1. Bind globals: `wl_compositor`, `wl_shm` (advertised), `wl_seat`,
//!    `wl_output`, `zxdg_shell_v6`, `wtz_shell`.
//! 2. `wl_compositor.create_surface()` → `wl_surface`.
//! 3. `zxdg_shell_v6.get_xdg_surface(wl_surface)` → `zxdg_surface_v6`.
//! 4. `zxdg_surface_v6.get_toplevel()` → `zxdg_toplevel_v6`; set title
//!    + app_id.
//! 5. `wtz_shell.get_wtz_surface(wl_surface)` → `wtz_surface`.
//! 6. Initial commit to trigger the compositor's first `configure`.
//! 7. On configure: ack, allocate a TBM-backed `wl_buffer` of the
//!    suggested size, fill, attach, damage, commit.
//! 8. Spin the event queue until the compositor sends `toplevel.close`.

#![warn(missing_docs)]

mod display;
mod error;
mod event;
mod event_loop;
mod window;
mod xkb;

pub use display::Display;
pub use error::{Error, Result};
pub use event::{Event, ModifiersState, MouseButton};
pub use event_loop::EventLoop;
pub use window::{keys, KeyGrab, KeyGrabMode, Window, WindowBuilder, WindowType};

// Re-export `raw-window-handle` so downstream apps can take the trait
// without an extra `Cargo.toml` line.
pub use raw_window_handle;
