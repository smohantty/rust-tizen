//! Events delivered by the window/display dispatcher.
//!
//! Variant names and field shapes mirror `winit::event::WindowEvent`
//! so anyone already familiar with winit needs zero new mental model.

// Field names inside the Event variants (width, height, x, y, pressed,
// modifiers, …) mirror winit's and are self-explanatory; we don't
// add a doc comment per field to keep this enum scannable.
#![allow(missing_docs)]

use bitflags::bitflags;

/// One event the compositor (or our own dispatcher) has produced for
/// the window. Drained by `Display::run` and delivered to the user
/// closure.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// The compositor resized us (or accepted our initial size).
    /// Mirrors `winit::event::WindowEvent::Resized`.
    Resized { width: u32, height: u32 },

    /// A previously-requested frame callback fired — time to paint
    /// the next frame. Mirrors `winit::event::WindowEvent::RedrawRequested`.
    RedrawRequested,

    /// User clicked the close box / compositor sent toplevel.close.
    /// Mirrors `winit::event::WindowEvent::CloseRequested`.
    CloseRequested,

    /// Keyboard focus gained/lost. Mirrors `Focused(bool)`.
    Focused(bool),

    /// Pointer entered our surface.
    /// Mirrors `winit::event::WindowEvent::CursorEntered`.
    CursorEntered { x: f64, y: f64 },

    /// Pointer left our surface.
    /// Mirrors `winit::event::WindowEvent::CursorLeft`.
    CursorLeft,

    /// Pointer moved while inside our surface.
    /// Mirrors `winit::event::WindowEvent::CursorMoved`.
    CursorMoved { x: f64, y: f64 },

    /// A mouse button was pressed or released.
    /// Mirrors `winit::event::WindowEvent::MouseInput`.
    MouseInput { button: MouseButton, pressed: bool },

    /// Scroll wheel motion. `dx`/`dy` are in "logical lines" (not pixels).
    /// Mirrors `winit::event::WindowEvent::MouseWheel`.
    MouseWheel { dx: f64, dy: f64 },

    /// A key was pressed or released. `keycode` is a raw evdev keycode
    /// (Linux input-event-codes.h) — no xkbcommon translation in v1.
    /// Mirrors `winit::event::WindowEvent::KeyboardInput`.
    KeyboardInput {
        keycode: u32,
        pressed: bool,
        modifiers: ModifiersState,
    },
}

/// Mouse button enum — variants match `winit::event::MouseButton`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MouseButton {
    /// Primary / left.
    Left,
    /// Secondary / right.
    Right,
    /// Middle.
    Middle,
    /// Back (forward = next page in browsers).
    Back,
    /// Forward.
    Forward,
    /// Any other button — raw linux/input-event-codes.h value.
    Other(u16),
}

bitflags! {
    /// Modifier-key state — bit positions match `winit::keyboard::ModifiersState`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ModifiersState: u32 {
        /// Shift (either side).
        const SHIFT = 1 << 0;
        /// Control (either side).
        const CTRL  = 1 << 1;
        /// Alt (either side).
        const ALT   = 1 << 2;
        /// Logo / Super / Windows / Meta key.
        const LOGO  = 1 << 3;
    }
}
