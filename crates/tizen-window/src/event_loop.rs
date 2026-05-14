//! Calloop-based event loop integrating wayland-fd dispatch with
//! SIGINT/SIGTERM handling.
//!
//! Adopts the standard wayland-rs ecosystem stack — [`calloop`] is what
//! winit uses on Linux, and [`calloop_wayland_source::WaylandSource`] is
//! the official adapter for [`wayland_client::EventQueue`]. Adding
//! timers, IPC channels, or custom wakeups later is just another
//! `handle.insert_source(...)` call.

use std::time::Duration;

use calloop::signals::{Signal, Signals};
use calloop_wayland_source::WaylandSource;

use crate::display::Display;
use crate::error::{Error, Result};
use crate::window::{Window, WindowState};

/// Event loop combining wayland-fd polling and Unix-signal handling.
///
/// Construct with [`EventLoop::new`], passing the [`Display`] (which is
/// consumed — its connection and event queue are moved into the loop).
/// Run with [`EventLoop::run`], passing the [`Window`] and a per-tick
/// callback. The loop exits cleanly when the compositor sends close,
/// the user presses Ctrl-C, or the process receives SIGTERM.
pub struct EventLoop {
    inner: calloop::EventLoop<'static, WindowState>,
}

impl EventLoop {
    /// Build a calloop event loop that already has wayland and signal
    /// sources registered. Consumes the [`Display`] — its connection
    /// and event queue move into the loop.
    pub fn new(display: Display) -> Result<Self> {
        let inner: calloop::EventLoop<'static, WindowState> =
            calloop::EventLoop::try_new().map_err(|e| Error::Transport(e.to_string()))?;
        let handle = inner.handle();

        let (conn, queue) = display.into_parts();
        WaylandSource::new(conn, queue)
            .insert(handle.clone())
            .map_err(|e| Error::Transport(e.to_string()))?;

        // Reset SIG_IGN dispositions inherited from the parent shell
        // before `Signals::new` blocks + signalfds these signals.
        // Background jobs (`cmd &`) inherit SIGINT/SIGQUIT as SIG_IGN
        // per POSIX job control; the kernel then discards those
        // signals without queueing them, so `signalfd` never reads
        // them. Setting disposition back to default makes the queued
        // delivery work.
        // SAFETY: `libc::signal` with `SIG_DFL` is signal-safe and only
        // changes the process-wide disposition.
        unsafe {
            libc::signal(libc::SIGINT, libc::SIG_DFL);
            libc::signal(libc::SIGTERM, libc::SIG_DFL);
        }

        let signals = Signals::new(&[Signal::SIGINT, Signal::SIGTERM])
            .map_err(|e| Error::Transport(e.to_string()))?;
        handle
            .insert_source(signals, |_event, _meta, state: &mut WindowState| {
                state.should_close = true;
            })
            .map_err(|e| Error::Transport(e.to_string()))?;

        Ok(Self { inner })
    }

    /// Run the loop until the window should close. Each iteration the
    /// loop dispatches any ready sources (wayland events, signals) and
    /// then invokes `on_tick` with `&mut Window` so the caller can
    /// drain queued [`crate::Event`]s and render the next frame.
    ///
    /// Generic over the caller's error type so apps and integration
    /// crates can return their own `Result<(), E>` from `on_tick`;
    /// transport errors from the loop itself are converted into `E`
    /// via the `From<Error>` bound. Returns the [`Window`] so the
    /// caller can do explicit teardown after the loop exits.
    pub fn run<F, E>(mut self, mut window: Window, mut on_tick: F) -> std::result::Result<Window, E>
    where
        F: FnMut(&mut Window) -> std::result::Result<(), E>,
        E: From<Error>,
    {
        // Cap each blocking wait at ~one frame so we still tick the
        // app even when no wayland events arrive (e.g. animation via
        // `continuous_repaint`). Wayland's `wl_surface.frame` callback
        // usually wakes us sooner; this is the idle-safety bound.
        let max_wait = Duration::from_millis(16);
        loop {
            self.inner
                .dispatch(Some(max_wait), window.state_mut())
                .map_err(|e| Error::Transport(e.to_string()))?;
            if window.should_close() {
                return Ok(window);
            }
            on_tick(&mut window)?;
        }
    }
}
