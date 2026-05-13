use std::{thread, time::Duration};

use crate::error::{Error, ProtocolError, Result};

use tizen_input_sys::protocol::tizen_input_device::TizenInputDevice;
use tizen_input_sys::protocol::tizen_input_device_manager::{
    self as tidm, Clas, Event as MgrEvent, PointerEventType, TizenInputDeviceManager,
};
use wayland_client::protocol::{wl_registry, wl_seat::WlSeat};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

/// Map our bitflags into the scanner-generated `Clas` bitflags.
fn clas_for(devices: DeviceType) -> Clas {
    let mut c = Clas::empty();
    if devices.contains(DeviceType::POINTER) {
        c |= Clas::Mouse;
    }
    if devices.contains(DeviceType::KEYBOARD) {
        c |= Clas::Keyboard;
    }
    if devices.contains(DeviceType::TOUCHSCREEN) {
        c |= Clas::Touchscreen;
    }
    c
}

bitflags::bitflags! {
    /// Device classes a generator can pretend to be. Matches the protocol's
    /// `clas` enum bitmask.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct DeviceType: u32 {
        /// Pointer / mouse device.
        const POINTER     = 1;
        /// Keyboard device.
        const KEYBOARD    = 2;
        /// Touchscreen device.
        const TOUCHSCREEN = 4;
    }
}

/// Whether a key event represents press or release.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum KeyState {
    /// Press (`pressed = 1` on the wire).
    Pressed,
    /// Release (`pressed = 0` on the wire).
    Released,
}

impl KeyState {
    fn as_u32(self) -> u32 {
        match self {
            Self::Pressed => 1,
            Self::Released => 0,
        }
    }
}

/// Phase of a touch event. Maps to the protocol's `pointer_event_type` enum.
///
/// A simulated tap is `Begin` → (small delay) → `End` at the same coordinate;
/// a drag is `Begin` → one or more `Update`s → `End`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TouchPhase {
    /// Finger lands at (x, y).
    Begin,
    /// Finger has moved to (x, y) while still pressed.
    Update,
    /// Finger lifts at (x, y).
    End,
}

impl TouchPhase {
    fn to_proto(self) -> PointerEventType {
        match self {
            Self::Begin => PointerEventType::Begin,
            Self::Update => PointerEventType::Update,
            Self::End => PointerEventType::End,
        }
    }
}

/// Phase of a pointer event. Same wire enum as touch (the protocol reuses
/// `pointer_event_type` for both), but the semantics differ:
///
/// * `ButtonDown` — a button is pressed at (x, y).
/// * `Move` — the pointer has moved to (x, y) (button state unchanged).
/// * `ButtonUp` — a button is released at (x, y).
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PointerPhase {
    /// Button press at (x, y).
    ButtonDown,
    /// Cursor move to (x, y), no button state change.
    Move,
    /// Button release at (x, y).
    ButtonUp,
}

impl PointerPhase {
    fn to_proto(self) -> PointerEventType {
        match self {
            Self::ButtonDown => PointerEventType::Begin,
            Self::Move => PointerEventType::Update,
            Self::ButtonUp => PointerEventType::End,
        }
    }
}

/// Mouse button index sent in the `button` field of `generate_pointer`.
///
/// Tizen's input-device-manager uses **X11-style button indices** (1 = left,
/// 2 = middle, 3 = right), *not* Linux input-event-codes (BTN_LEFT = 0x110)
/// nor a bitmask. Confirmed by the upstream `efl_util` test suite
/// (`tc-efl-util-internal.cpp:269+` passes `1` for left-button events).
/// Passing `0x110` is rejected with `invalid_parameter`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum PointerButton {
    /// Primary/left button (X11 button 1). Used by [`InputGenerator::click`].
    Left,
    /// Middle button (X11 button 2).
    Middle,
    /// Secondary/right button (X11 button 3).
    Right,
    /// Raw button index passthrough — for non-standard buttons supported
    /// by a specific Tizen profile. Values 4/5 are wheel up/down in X11.
    Other(u32),
}

impl PointerButton {
    fn as_u32(self) -> u32 {
        match self {
            Self::Left => 1,
            Self::Middle => 2,
            Self::Right => 3,
            Self::Other(code) => code,
        }
    }
}

/// Builder for [`InputGenerator`]. Obtain via [`InputGenerator::builder`].
#[derive(Debug, Clone)]
pub struct InputGeneratorBuilder {
    devices: DeviceType,
    name: Option<String>,
}

impl InputGeneratorBuilder {
    /// Which device classes to register. Combine flags with `|`.
    /// Default: [`DeviceType::KEYBOARD`].
    pub fn device(mut self, devices: DeviceType) -> Self {
        self.devices = devices;
        self
    }

    /// Optional identifier sent to the compositor. Servers may log this in
    /// `dlogutil` output for debugging. Defaults to the protocol-defined
    /// default ("Input Generator") if omitted.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Open the connection, bind `tizen_input_device_manager`, and initialise
    /// the requested generator classes. The returned [`InputGenerator`]
    /// holds the generator open until dropped.
    pub fn open(self) -> Result<InputGenerator> {
        InputGenerator::open_inner(self.devices, self.name)
    }
}

/// A live input generator. While this value is alive the process has a
/// virtual device of the requested class(es) attached to the compositor.
///
/// Dropping releases the generator (`deinit_generator`) and disconnects.
pub struct InputGenerator {
    /// `None` only in host fallback mode where no compositor is reachable.
    inner: Option<Inner>,
    devices: DeviceType,
}

struct Inner {
    conn: Connection,
    queue: EventQueue<State>,
    mgr: TizenInputDeviceManager,
    state: State,
}

impl InputGenerator {
    /// Open a generator with default settings (keyboard only, no custom name).
    pub fn open() -> Result<Self> {
        Self::builder().open()
    }

    /// Start a builder.
    pub fn builder() -> InputGeneratorBuilder {
        InputGeneratorBuilder {
            devices: DeviceType::KEYBOARD,
            name: None,
        }
    }

    /// Inject a key event. `name` is an X11-style keysym name (e.g.
    /// `"XF86Back"`, `"KEY_VOLUMEUP"`, `"Return"`) — the compositor maps
    /// it to a keycode using its own table.
    pub fn key(&mut self, name: &str, state: KeyState) -> Result<()> {
        if !self.devices.contains(DeviceType::KEYBOARD) {
            return Err(Error::Rejected(ProtocolError::InvalidClass));
        }
        let Some(inner) = self.inner.as_mut() else {
            eprintln!("tizen-input(host): key {name} {state:?}");
            return Ok(());
        };
        inner.mgr.generate_key(name.to_owned(), state.as_u32());
        inner.dispatch_until_ack()
    }

    /// Inject a touch event for finger `idx` at screen coordinates `(x, y)`.
    ///
    /// The generator must have been opened with [`DeviceType::TOUCHSCREEN`].
    /// `idx` is the finger slot (0-based) and must be below the compositor's
    /// `max_touch_count` (typically 10–20 on Tizen TV / mobile).
    ///
    /// Use [`InputGenerator::tap`] for the common begin → end shortcut.
    pub fn touch(&mut self, idx: u32, phase: TouchPhase, x: u32, y: u32) -> Result<()> {
        if !self.devices.contains(DeviceType::TOUCHSCREEN) {
            return Err(Error::Rejected(ProtocolError::InvalidClass));
        }
        let Some(inner) = self.inner.as_mut() else {
            eprintln!("tizen-input(host): touch idx={idx} {phase:?} at ({x},{y})");
            return Ok(());
        };
        if let Some(max) = inner.state.max_touch_count {
            if idx >= max {
                return Err(Error::Rejected(ProtocolError::InvalidParameter));
            }
        }
        inner.mgr.generate_touch(phase.to_proto(), x, y, idx);
        inner.dispatch_until_ack()
    }

    /// Convenience wrapper for a tap: `Begin → 50ms hold → End` at the same
    /// coordinate on finger slot `idx`. Equivalent to two [`Self::touch`]
    /// calls.
    pub fn tap(&mut self, idx: u32, x: u32, y: u32) -> Result<()> {
        self.touch(idx, TouchPhase::Begin, x, y)?;
        thread::sleep(Duration::from_millis(50));
        self.touch(idx, TouchPhase::End, x, y)
    }

    /// Inject a pointer event at screen coordinates `(x, y)`.
    ///
    /// The generator must have been opened with [`DeviceType::POINTER`].
    /// `button` is the linux/input-event-codes button code (e.g.
    /// [`PointerButton::Left`] = `BTN_LEFT`); the compositor uses it for
    /// `ButtonDown`/`ButtonUp` phases and typically ignores it for `Move`.
    pub fn pointer(
        &mut self,
        button: PointerButton,
        phase: PointerPhase,
        x: u32,
        y: u32,
    ) -> Result<()> {
        if !self.devices.contains(DeviceType::POINTER) {
            return Err(Error::Rejected(ProtocolError::InvalidClass));
        }
        let Some(inner) = self.inner.as_mut() else {
            eprintln!("tizen-input(host): pointer {button:?} {phase:?} at ({x},{y})");
            return Ok(());
        };
        inner
            .mgr
            .generate_pointer(phase.to_proto(), x, y, button.as_u32());
        inner.dispatch_until_ack()
    }

    /// Convenience wrapper for a click: `Move` to (x, y) → `ButtonDown` →
    /// 50ms hold → `ButtonUp`. Uses the given mouse button.
    ///
    /// On Tizen TV this is the visible analogue of `tap` — it moves the
    /// on-screen pointer to `(x, y)` and presses the primary button.
    pub fn click(&mut self, button: PointerButton, x: u32, y: u32) -> Result<()> {
        self.pointer(button, PointerPhase::Move, x, y)?;
        self.pointer(button, PointerPhase::ButtonDown, x, y)?;
        thread::sleep(Duration::from_millis(50));
        self.pointer(button, PointerPhase::ButtonUp, x, y)
    }

    /// Move the pointer to `(x, y)` without changing any button state.
    /// Equivalent to `self.pointer(PointerButton::Left, PointerPhase::Move, x, y)`.
    pub fn move_pointer(&mut self, x: u32, y: u32) -> Result<()> {
        self.pointer(PointerButton::Left, PointerPhase::Move, x, y)
    }

    /// Most recent `max_touch_count` reported by the compositor for this
    /// connection, or `None` if no event has arrived yet (rare; the server
    /// emits it during `init_generator` ack).
    pub fn max_touch_count(&self) -> Option<u32> {
        self.inner.as_ref().and_then(|i| i.state.max_touch_count)
    }

    fn open_inner(devices: DeviceType, name: Option<String>) -> Result<Self> {
        if devices.is_empty() {
            return Err(Error::Rejected(ProtocolError::InvalidClass));
        }

        // Connect — on host without $WAYLAND_DISPLAY this errors cleanly and
        // we fall through to the no-op generator.
        let conn = match Connection::connect_to_env() {
            Ok(c) => c,
            Err(e) => {
                if cfg!(tizen) {
                    return Err(Error::NotConnected(e.to_string()));
                }
                eprintln!(
                    "tizen-input(host): no compositor reachable ({e}); using no-op generator"
                );
                return Ok(Self {
                    inner: None,
                    devices,
                });
            }
        };

        let display = conn.display();
        let mut queue: EventQueue<State> = conn.new_event_queue();
        let qh = queue.handle();

        let mut state = State::default();

        // First roundtrip: enumerate globals.
        let _registry = display.get_registry(&qh, ());
        queue
            .roundtrip(&mut state)
            .map_err(|e| Error::Transport(e.to_string()))?;

        let mgr = state.mgr.clone().ok_or(Error::ProtocolUnavailable)?;

        // Initialise the generator. The server replies asynchronously via
        // the manager's `error` event (errorcode=0 means success).
        let clas = clas_for(devices);
        match name {
            Some(n) => mgr.init_generator_with_name(clas, n),
            None => mgr.init_generator(clas),
        }

        let mut inner = Inner {
            conn,
            queue,
            mgr,
            state,
        };
        inner.dispatch_until_ack()?;

        Ok(Self {
            inner: Some(inner),
            devices,
        })
    }
}

impl Inner {
    /// Spin the event queue until the manager fires its `error` event,
    /// then translate it to a `Result`. The protocol uses `error` as a
    /// generic ack — code 0 means success.
    fn dispatch_until_ack(&mut self) -> Result<()> {
        loop {
            self.queue
                .blocking_dispatch(&mut self.state)
                .map_err(|e| Error::Transport(e.to_string()))?;
            if let Some(code) = self.state.error.take() {
                return if code == 0 {
                    Ok(())
                } else {
                    Err(Error::Rejected(ProtocolError::from_code(code)))
                };
            }
        }
    }
}

impl Drop for InputGenerator {
    fn drop(&mut self) {
        let Some(inner) = self.inner.as_mut() else {
            return;
        };
        inner.mgr.deinit_generator(clas_for(self.devices));
        // Best-effort flush; failure during teardown is logged, not returned.
        if let Err(e) = inner.queue.roundtrip(&mut inner.state) {
            eprintln!("tizen-input: drop roundtrip failed: {e}");
        }
        let _ = inner.conn.flush();
    }
}

/// Internal dispatch state. Only the `tizen_input_device_manager` global is
/// captured during the registry roundtrip; `wl_seat` is captured so the
/// compositor's `device_add` events have a target to attach to.
#[derive(Default)]
struct State {
    mgr: Option<TizenInputDeviceManager>,
    /// Latest error code from the manager — `Some(0)` means success ack.
    error: Option<u32>,
    /// Compositor-advertised maximum finger slot count for touch generation,
    /// captured from the manager's `max_touch_count` event.
    max_touch_count: Option<u32>,
    _seat: Option<WlSeat>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
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
            "tizen_input_device_manager" => {
                let mgr = registry.bind::<TizenInputDeviceManager, _, _>(name, version, qh, ());
                state.mgr = Some(mgr);
            }
            "wl_seat" => {
                let seat = registry.bind::<WlSeat, _, _>(name, version.min(7), qh, ());
                state._seat = Some(seat);
            }
            _ => {}
        }
    }
}

impl Dispatch<TizenInputDeviceManager, ()> for State {
    fn event(
        state: &mut Self,
        _: &TizenInputDeviceManager,
        event: MgrEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            MgrEvent::Error { errorcode } => state.error = Some(errorcode.into()),
            MgrEvent::MaxTouchCount { max_count, .. } => {
                state.max_touch_count = Some(max_count.max(0) as u32);
            }
            MgrEvent::DeviceAdd { .. }
            | MgrEvent::DeviceRemove { .. }
            | MgrEvent::BlockExpired
            | MgrEvent::EventBoundary { .. } => {}
            _ => {}
        }
    }

    // The `device_add` event creates a `tizen_input_device` proxy via
    // `new_id`. wayland-client needs us to declare the child user-data
    // type per parent-event opcode, otherwise it panics on first delivery.
    wayland_client::event_created_child!(State, TizenInputDeviceManager, [
        tidm::EVT_DEVICE_ADD_OPCODE => (TizenInputDevice, ()),
    ]);
}

impl Dispatch<TizenInputDevice, ()> for State {
    fn event(
        _: &mut Self,
        _: &TizenInputDevice,
        _: <TizenInputDevice as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: <WlSeat as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
