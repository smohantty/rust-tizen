use crate::error::{Error, ProtocolError, Result};

use tizen_input_sys::protocol::tizen_input_device::TizenInputDevice;
use tizen_input_sys::protocol::tizen_input_device_manager::{
    Clas, Event as MgrEvent, TizenInputDeviceManager,
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
        let Some(inner) = self.inner.as_mut() else {
            eprintln!("tizen-input(host): key {name} {state:?}");
            return Ok(());
        };
        inner.mgr.generate_key(name.to_owned(), state.as_u32());
        inner.dispatch_until_ack()
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

        let mut state = State {
            mgr: None,
            error: None,
            _seat: None,
        };

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
            MgrEvent::DeviceAdd { .. }
            | MgrEvent::DeviceRemove { .. }
            | MgrEvent::BlockExpired
            | MgrEvent::MaxTouchCount { .. }
            | MgrEvent::EventBoundary { .. } => {}
            _ => {}
        }
    }
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
