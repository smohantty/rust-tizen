use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Arc;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, RawDisplayHandle, WaylandDisplayHandle,
};
use tizen_tbm_sys::wayland_tbm;
use tizen_window_sys::tizen_extension::tizen_policy::TizenPolicy;
use tizen_window_sys::wtz_shell::wtz_shell::WtzShell;
use tizen_window_sys::xdg_shell_v6::zxdg_shell_v6::ZxdgShellV6;
use wayland_backend::client::{Backend, ObjectData, ObjectId};
use wayland_backend::protocol::Message;
use wayland_client::protocol::{
    wl_compositor::WlCompositor,
    wl_keyboard::WlKeyboard,
    wl_pointer::WlPointer,
    wl_registry,
    wl_seat::{self, WlSeat},
};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};

use crate::error::{Error, Result};
use crate::window::WindowState;

/// A connected Tizen Wayland display with the globals an MVP window
/// needs already bound.
pub struct Display {
    pub(crate) conn: Connection,
    pub(crate) queue: EventQueue<WindowState>,
    pub(crate) compositor: WlCompositor,
    pub(crate) xdg_shell: ZxdgShellV6,
    pub(crate) wtz_shell: WtzShell,
    pub(crate) tz_policy: Option<TizenPolicy>,
    pub(crate) tbm_client: TbmClientHandle,
}

/// Owns the dlopen'd `wayland_tbm_client *` and deinits on drop.
pub(crate) struct TbmClientHandle {
    pub(crate) ptr: *mut wayland_tbm::wayland_tbm_client,
}

impl Drop for TbmClientHandle {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { wayland_tbm::deinit(self.ptr) };
            self.ptr = core::ptr::null_mut();
        }
    }
}

impl Display {
    /// Connect to `$WAYLAND_DISPLAY`, bind all globals an MVP window
    /// needs, and initialise the TBM client. On host builds without
    /// a reachable Tizen compositor this returns [`Error::Unsupported`].
    pub fn connect() -> Result<Self> {
        let conn = Connection::connect_to_env().map_err(|e| Error::NotConnected(e.to_string()))?;

        let display_proxy = conn.display();
        let mut queue: EventQueue<WindowState> = conn.new_event_queue();
        let qh = queue.handle();

        let mut state = WindowState::default();

        // Drive the registry once to capture every global we care about.
        let _registry = display_proxy.get_registry(&qh, ());
        queue
            .roundtrip(&mut state)
            .map_err(|e| Error::Transport(e.to_string()))?;

        let compositor = state
            .compositor
            .clone()
            .ok_or(Error::GlobalMissing("wl_compositor"))?;
        let xdg_shell = state
            .xdg_shell
            .clone()
            .ok_or(Error::GlobalMissing("zxdg_shell_v6"))?;
        let wtz_shell = state
            .wtz_shell
            .clone()
            .ok_or(Error::GlobalMissing("wtz_shell"))?;
        // tizen_policy is optional from a "create a surface" standpoint
        // but mandatory for actually making the window visible on Tizen
        // (the compositor places client surfaces below the launcher
        // until tizen_policy.show / .activate is called).
        let tz_policy = state.tz_policy.clone();

        // SAFETY: `Backend::display_ptr()` returns a live `wl_display *`
        // for the connection's lifetime.
        let tbm_ptr = unsafe {
            let display_ptr = conn.backend().display_ptr() as *mut c_void;
            wayland_tbm::init(display_ptr).map_err(|e| Error::Tbm(e.to_string()))?
        };
        if tbm_ptr.is_null() {
            return Err(Error::Tbm("wayland_tbm_client_init returned NULL".into()));
        }

        Ok(Self {
            conn,
            queue,
            compositor,
            xdg_shell,
            wtz_shell,
            tz_policy,
            tbm_client: TbmClientHandle { ptr: tbm_ptr },
        })
    }

    /// Drive one round of event dispatch — non-blocking equivalent of
    /// "wait for the compositor, then react".
    pub fn dispatch_pending(&mut self, window: &mut crate::Window) -> Result<()> {
        self.queue
            .blocking_dispatch(&mut window.state)
            .map_err(|e| Error::Transport(e.to_string()))?;
        Ok(())
    }

    /// Force a roundtrip — useful right after creating the window to
    /// receive the initial `configure`.
    pub fn roundtrip(&mut self, window: &mut crate::Window) -> Result<()> {
        self.queue
            .roundtrip(&mut window.state)
            .map_err(|e| Error::Transport(e.to_string()))?;
        Ok(())
    }

    /// Drive the event loop, calling `callback` with each
    /// [`crate::Event`] until the compositor sends close. Drains any
    /// events queued before the first dispatch (e.g. the initial
    /// `RedrawRequested` from the first configure) before blocking.
    pub fn run<F>(&mut self, window: &mut crate::Window, mut callback: F) -> Result<()>
    where
        F: FnMut(&mut crate::Window, crate::Event),
    {
        loop {
            let pending: Vec<crate::Event> = window.drain_events().collect();
            for ev in pending {
                callback(window, ev);
            }
            if window.should_close() {
                return Ok(());
            }
            self.queue
                .blocking_dispatch(&mut window.state)
                .map_err(|e| Error::Transport(e.to_string()))?;
        }
    }

    pub(crate) fn queue_handle(&self) -> QueueHandle<WindowState> {
        self.queue.handle()
    }
}

// ----- raw-window-handle interop -------------------------------------------
//
// `HasDisplayHandle` lets any renderer (softbuffer, glow via khronos-egl,
// wgpu, ash, …) discover our underlying `wl_display *` without us taking
// a dependency on those crates. This is the standard ecosystem trait
// (see `raw-window-handle` 0.6).

impl HasDisplayHandle for Display {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        // `Connection::backend()` returns a cheaply-cloned `Backend` (Arc inside);
        // `Backend::display_ptr()` is exposed when the `client_system` feature is on
        // (which our Cargo.toml enables).
        let display_ptr = self.conn.backend().display_ptr() as *mut c_void;
        let nn = NonNull::new(display_ptr).ok_or(HandleError::Unavailable)?;
        let raw = WaylandDisplayHandle::new(nn);
        // SAFETY: the `wl_display *` is valid for the lifetime of `Display`
        // (we hold the `Connection`); `DisplayHandle::borrow_raw` ties the
        // returned handle's lifetime to `&self`, so the pointer cannot
        // outlive the `Display`.
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Wayland(raw)) })
    }
}

// ---------------------------------------------------------------------------
// Dispatch impls for the globals — these live on the WindowState because
// it's the only `S` type we have. wayland-client requires Dispatch<T, U>
// for State : Dispatch.

impl Dispatch<wl_registry::WlRegistry, ()> for WindowState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };
        match interface.as_str() {
            "wl_compositor" => {
                state.compositor =
                    Some(registry.bind::<WlCompositor, _, _>(name, version.min(4), qh, ()));
            }
            "zxdg_shell_v6" => {
                state.xdg_shell =
                    Some(registry.bind::<ZxdgShellV6, _, _>(name, version.min(1), qh, ()));
            }
            "wtz_shell" => {
                state.wtz_shell =
                    Some(registry.bind::<WtzShell, _, _>(name, version.min(1), qh, ()));
            }
            "tizen_policy" => {
                // We need at least v8 for `tizen_policy.show`; clamp to
                // 8 so we get exactly the bindings our XML declares.
                state.tz_policy =
                    Some(registry.bind::<TizenPolicy, _, _>(name, version.min(8), qh, ()));
            }
            "wl_seat" => {
                state.seat = Some(registry.bind::<WlSeat, _, _>(name, version.min(7), qh, ()));
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for WindowState {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: <WlCompositor as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for WindowState {
    fn event(
        state: &mut Self,
        seat: &WlSeat,
        event: <WlSeat as Proxy>::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(caps),
        } = event
        {
            if caps.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
            if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                state.keyboard = Some(seat.get_keyboard(qh, ()));
            }
        }
    }
}

// `WlPointer` lives on the same dispatch state as the rest of the
// window. The compositor only delivers pointer events for surfaces we
// own, so we don't filter by `surface` id today.
impl Dispatch<WlPointer, ()> for WindowState {
    fn event(
        state: &mut Self,
        _: &WlPointer,
        event: <WlPointer as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use wayland_client::protocol::wl_pointer::{Axis, ButtonState, Event};
        match event {
            Event::Enter {
                surface_x,
                surface_y,
                ..
            } => state.pending_events.push(crate::Event::CursorEntered {
                x: surface_x,
                y: surface_y,
            }),
            Event::Leave { .. } => state.pending_events.push(crate::Event::CursorLeft),
            Event::Motion {
                surface_x,
                surface_y,
                ..
            } => state.pending_events.push(crate::Event::CursorMoved {
                x: surface_x,
                y: surface_y,
            }),
            Event::Button {
                button,
                state: btn_state,
                ..
            } => {
                let pressed = matches!(btn_state, WEnum::Value(ButtonState::Pressed));
                state.pending_events.push(crate::Event::MouseInput {
                    button: evdev_to_mouse_button(button),
                    pressed,
                });
            }
            Event::Axis { axis, value, .. } => {
                let (dx, dy) = match axis {
                    WEnum::Value(Axis::HorizontalScroll) => (value, 0.0),
                    WEnum::Value(Axis::VerticalScroll) => (0.0, value),
                    _ => (0.0, 0.0),
                };
                state
                    .pending_events
                    .push(crate::Event::MouseWheel { dx, dy });
            }
            _ => {}
        }
    }
}

// Raw keycodes via wl_keyboard. We don't ship xkbcommon yet, so the
// `keymap` event is consumed-and-dropped (closing the fd) and the
// `modifiers` event is ignored — `Event::KeyboardInput` always
// surfaces `modifiers: ModifiersState::empty()` until a future
// xkbcommon pass translates mod indices.
impl Dispatch<WlKeyboard, ()> for WindowState {
    fn event(
        state: &mut Self,
        _: &WlKeyboard,
        event: <WlKeyboard as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use wayland_client::protocol::wl_keyboard::{Event, KeyState};
        match event {
            Event::Enter { .. } => state.pending_events.push(crate::Event::Focused(true)),
            Event::Leave { .. } => state.pending_events.push(crate::Event::Focused(false)),
            Event::Key {
                key,
                state: key_state,
                ..
            } => {
                let pressed = matches!(key_state, WEnum::Value(KeyState::Pressed));
                state.pending_events.push(crate::Event::KeyboardInput {
                    keycode: key,
                    pressed,
                    modifiers: crate::ModifiersState::empty(),
                });
            }
            _ => {}
        }
    }
}

fn evdev_to_mouse_button(code: u32) -> crate::MouseButton {
    use crate::MouseButton;
    match code {
        0x110 => MouseButton::Left,    // BTN_LEFT
        0x111 => MouseButton::Right,   // BTN_RIGHT
        0x112 => MouseButton::Middle,  // BTN_MIDDLE
        0x113 => MouseButton::Back,    // BTN_SIDE
        0x114 => MouseButton::Forward, // BTN_EXTRA
        other => MouseButton::Other(other as u16),
    }
}

// ---------------------------------------------------------------------------
// `wl_buffer` adoption — wraps a raw `wl_buffer *` from
// libwayland-tbm-client into a wayland-client `WlBuffer` proxy so our
// scanner-generated `surface.attach(&buf, ...)` call can use it.

pub(crate) struct NoopData;
impl ObjectData for NoopData {
    fn event(
        self: Arc<Self>,
        _: &Backend,
        _: Message<ObjectId, std::os::fd::OwnedFd>,
    ) -> Option<Arc<dyn ObjectData>> {
        None
    }
    fn destroyed(&self, _: ObjectId) {}
}

pub(crate) unsafe fn adopt_wl_buffer(
    conn: &Connection,
    ptr: *mut c_void,
) -> Result<wayland_client::protocol::wl_buffer::WlBuffer> {
    use wayland_client::protocol::wl_buffer::WlBuffer;
    let backend = conn.backend();
    let id =
        unsafe { backend.manage_object(WlBuffer::interface(), ptr as *mut _, Arc::new(NoopData)) };
    WlBuffer::from_id(conn, id).map_err(|e| Error::Tbm(format!("WlBuffer::from_id: {e:?}")))
}
