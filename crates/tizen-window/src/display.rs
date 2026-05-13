use std::ffi::c_void;
use std::sync::Arc;

use tizen_tbm_sys::wayland_tbm;
use tizen_window_sys::tizen_extension::tizen_policy::TizenPolicy;
use tizen_window_sys::wtz_shell::wtz_shell::WtzShell;
use tizen_window_sys::xdg_shell_v6::zxdg_shell_v6::ZxdgShellV6;
use wayland_backend::client::{Backend, ObjectData, ObjectId};
use wayland_backend::protocol::Message;
use wayland_client::protocol::{wl_compositor::WlCompositor, wl_registry, wl_seat::WlSeat};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

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

    pub(crate) fn queue_handle(&self) -> QueueHandle<WindowState> {
        self.queue.handle()
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
        _: &mut Self,
        _: &WlSeat,
        _: <WlSeat as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
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
