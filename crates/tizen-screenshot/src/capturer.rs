use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::Arc;

use crate::error::{Error, Result};

use tizen_screenshot_sys::protocol::tizen_screenshooter::{
    Event as ShooterEvent, TizenScreenshooter,
};
use tizen_screenshot_sys::tbm;
use tizen_screenshot_sys::wayland_tbm;
use wayland_backend::client::{Backend, ObjectData, ObjectId};
use wayland_backend::protocol::Message;
use wayland_client::protocol::{wl_buffer::WlBuffer, wl_output::WlOutput, wl_registry};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

/// Minimum `tizen_screenshooter` version we need: `shoot` + `done` arrived
/// in v3; `area_shoot` is gated separately at call time.
const REQUIRED_VERSION: u32 = 3;

/// Format we ask the compositor to fill: `XRGB8888` little-endian.
///
/// On the wire this is 4 bytes per pixel; the X (high) byte is unused.
/// `ScreenFrame::bytes()` returns the buffer in this layout.
pub const SHOT_FORMAT: u32 = tbm::TBM_FORMAT_XRGB8888;

/// A handle that holds open the wayland connection + TBM client. While
/// alive, screenshots can be taken via [`Self::shoot`] / [`Self::shoot_area`].
///
/// Dropping releases the TBM client, the protocol object, and the
/// connection.
pub struct ScreenCapturer {
    inner: Inner,
}

struct Inner {
    conn: Connection,
    queue: EventQueue<State>,
    state: State,
    shooter: TizenScreenshooter,
    tbm_client: *mut wayland_tbm::wayland_tbm_client,
}

/// A captured screen image. Owns its pixel data — the TBM buffer it was
/// copied out of has already been released by the time the frame is
/// returned to the caller.
#[derive(Debug, Clone)]
pub struct ScreenFrame {
    width: u32,
    height: u32,
    /// Bytes per row including any padding. May be larger than
    /// `width * 4` if the compositor's buffer is row-aligned.
    stride: u32,
    /// Pixel data in [`SHOT_FORMAT`] order.
    bytes: Vec<u8>,
}

impl ScreenFrame {
    /// Frame width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Frame height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Bytes per row in the raw buffer. Use this when reading rows
    /// individually — `width * 4` is **not** always correct.
    pub fn stride(&self) -> u32 {
        self.stride
    }

    /// Format fourcc. Always [`SHOT_FORMAT`] for now.
    pub fn format(&self) -> u32 {
        SHOT_FORMAT
    }

    /// Raw pixel bytes in `XRGB8888` little-endian order. Length is
    /// `stride * height`.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the frame and return its owned byte buffer.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl ScreenCapturer {
    /// Connect to the compositor, bind the screenshooter / output globals,
    /// and initialise the TBM client.
    pub fn new() -> Result<Self> {
        if !cfg!(tizen) && !is_host_likely_tizen() {
            return Err(Error::Unsupported);
        }
        Self::open()
    }

    fn open() -> Result<Self> {
        let conn = Connection::connect_to_env().map_err(|e| Error::NotConnected(e.to_string()))?;

        let display = conn.display();
        let mut queue: EventQueue<State> = conn.new_event_queue();
        let qh = queue.handle();

        let mut state = State::default();

        // Registry roundtrip — capture the screenshooter + output globals.
        let _registry = display.get_registry(&qh, ());
        queue
            .roundtrip(&mut state)
            .map_err(|e| Error::Transport(e.to_string()))?;

        let shooter = state.shooter.clone().ok_or(Error::ProtocolUnavailable)?;

        let advertised_version = state.shooter_version.unwrap_or(0);
        if advertised_version < REQUIRED_VERSION {
            return Err(Error::ProtocolTooOld {
                advertised: advertised_version,
                required: REQUIRED_VERSION,
            });
        }

        if state.output.is_none() {
            return Err(Error::NoOutput);
        }

        // SAFETY: `conn.backend().display_ptr()` is a live `*mut wl_display`
        // tied to the `Connection`'s lifetime; `tbm_client` is null-checked below.
        // We intentionally do **not** call `wayland_tbm_client_set_event_queue`:
        // efl_util doesn't either. wl_tbm's release-event traffic flows to the
        // default queue, which is harmless because nothing in this crate blocks
        // on it — the screenshooter `done` event is what we wait on, and that
        // arrives on our own queue.
        let tbm_client = unsafe {
            let display_ptr = wl_display_ptr(&conn);
            wayland_tbm::init(display_ptr).map_err(|e| Error::TbmUnavailable(e.to_string()))?
        };
        if tbm_client.is_null() {
            return Err(Error::TbmUnavailable(
                "wayland_tbm_client_init returned NULL".into(),
            ));
        }

        Ok(Self {
            inner: Inner {
                conn,
                queue,
                state,
                shooter,
                tbm_client,
            },
        })
    }

    /// Capture the full screen at the compositor's native resolution
    /// (or the size you request — most compositors will scale-or-crop
    /// to match). Returns the pixel bytes.
    pub fn shoot(&mut self) -> Result<ScreenFrame> {
        let (w, h) = self.output_size();
        self.shoot_with_size(w, h)
    }

    /// Capture the full screen into a buffer of the requested `(w, h)`.
    /// Useful when you want a smaller-than-native screenshot (the
    /// compositor scales the framebuffer down).
    pub fn shoot_with_size(&mut self, w: u32, h: u32) -> Result<ScreenFrame> {
        self.capture_inner(w, h, None)
    }

    /// Capture a `(w, h)` rectangle starting at `(x, y)` (compositor pixels).
    /// Requires `tizen_screenshooter` v4+.
    pub fn shoot_area(&mut self, x: i32, y: i32, w: i32, h: i32) -> Result<ScreenFrame> {
        if w <= 0 || h <= 0 {
            return Err(Error::BufferAllocation(format!(
                "non-positive dimensions ({w}, {h})"
            )));
        }
        if self.inner.shooter.version() < 4 {
            return Err(Error::ProtocolTooOld {
                advertised: self.inner.shooter.version(),
                required: 4,
            });
        }
        self.capture_inner(w as u32, h as u32, Some((x, y, w, h)))
    }

    /// Toggle the compositor's "auto rotation" preference for one-shot
    /// captures. When set, the compositor rotates the captured buffer
    /// to match the screen orientation.
    pub fn set_auto_rotation(&mut self, on: bool) -> Result<()> {
        self.inner
            .shooter
            .set_oneshot_auto_rotation(if on { 1 } else { 0 });
        self.inner
            .queue
            .flush()
            .map_err(|e| Error::Transport(e.to_string()))?;
        Ok(())
    }

    /// Last-observed output size from `wl_output.mode` events, or a
    /// reasonable fallback if none arrived. Used by [`Self::shoot`].
    fn output_size(&self) -> (u32, u32) {
        match self.inner.state.output_size {
            Some((w, h)) if w > 0 && h > 0 => (w, h),
            _ => (1920, 1080),
        }
    }

    fn capture_inner(
        &mut self,
        w: u32,
        h: u32,
        area: Option<(i32, i32, i32, i32)>,
    ) -> Result<ScreenFrame> {
        // 1. Allocate a TBM-backed buffer the server can write into.
        let surface = unsafe { tbm::tbm_surface_create(w as c_int, h as c_int, SHOT_FORMAT) };
        if surface.is_null() {
            return Err(Error::BufferAllocation(format!(
                "tbm_surface_create({w}, {h}) returned NULL"
            )));
        }

        // Helper struct to free TBM resources on every exit path.
        struct BufferGuard {
            tbm_client: *mut wayland_tbm::wayland_tbm_client,
            surface: tbm::tbm_surface_h,
            wl_buf_ptr: *mut c_void,
        }
        impl Drop for BufferGuard {
            fn drop(&mut self) {
                unsafe {
                    if !self.wl_buf_ptr.is_null() {
                        wayland_tbm::destroy_buffer(self.tbm_client, self.wl_buf_ptr);
                    }
                    if !self.surface.is_null() {
                        tbm::tbm_surface_destroy(self.surface);
                    }
                }
            }
        }
        let mut guard = BufferGuard {
            tbm_client: self.inner.tbm_client,
            surface,
            wl_buf_ptr: core::ptr::null_mut(),
        };

        // 2. Wrap the TBM surface as a wl_buffer.
        let wl_buf_ptr = unsafe {
            wayland_tbm::create_buffer(self.inner.tbm_client, surface)
                .map_err(|e| Error::BufferAllocation(e.to_string()))?
        };
        if wl_buf_ptr.is_null() {
            return Err(Error::BufferAllocation(
                "wayland_tbm_client_create_buffer returned NULL".into(),
            ));
        }
        guard.wl_buf_ptr = wl_buf_ptr;

        // 3. Convert the raw `wl_buffer *` from libwayland-tbm-client into a
        //    wayland-client `WlBuffer` proxy we can pass to the screenshooter.
        let wl_buffer = unsafe { wl_buffer_from_ptr(&self.inner.conn, wl_buf_ptr)? };

        // 4. Fire the request.
        let output = self.inner.state.output.clone().ok_or(Error::NoOutput)?;
        self.inner.state.shot_done = false;
        self.inner.state.area_shot_done = false;
        match area {
            None => self.inner.shooter.shoot(&output, &wl_buffer),
            Some((x, y, w, h)) => self
                .inner
                .shooter
                .area_shoot(&output, &wl_buffer, x, y, w, h),
        }

        // 5. Spin until the matching `done` event arrives.
        let want_area = area.is_some();
        loop {
            self.inner
                .queue
                .blocking_dispatch(&mut self.inner.state)
                .map_err(|e| Error::Transport(e.to_string()))?;
            let done = if want_area {
                self.inner.state.area_shot_done
            } else {
                self.inner.state.shot_done
            };
            if done {
                break;
            }
        }

        // 6. Map the buffer, copy out, drop guard (frees TBM + wl_buffer).
        let frame = unsafe { copy_out(surface) }?;
        drop(guard);
        Ok(frame)
    }
}

impl Drop for ScreenCapturer {
    fn drop(&mut self) {
        // The TBM client is the only thing that owns FFI state; the
        // protocol proxy and connection clean themselves up when the
        // Rust handles go out of scope (the protocol `destroy` request
        // fires on TizenScreenshooter Drop via wayland-scanner).
        if !self.inner.tbm_client.is_null() {
            unsafe { wayland_tbm::deinit(self.inner.tbm_client) };
            self.inner.tbm_client = core::ptr::null_mut();
        }
    }
}

unsafe fn copy_out(surface: tbm::tbm_surface_h) -> Result<ScreenFrame> {
    let mut info = tbm::tbm_surface_info_s::default();
    let rc = unsafe { tbm::tbm_surface_map(surface, tbm::TBM_SURF_OPTION_READ, &mut info) };
    if rc != 0 {
        return Err(Error::BufferMap(format!("tbm_surface_map rc={rc}")));
    }

    let plane = info.planes[0];
    let stride = plane.stride.max(info.width * 4);
    let len = (stride as usize).saturating_mul(info.height as usize);
    let bytes = if plane.ptr.is_null() || len == 0 {
        unsafe { tbm::tbm_surface_unmap(surface) };
        return Err(Error::BufferMap(
            "tbm_surface_map produced a NULL plane pointer".into(),
        ));
    } else {
        // SAFETY: the buffer remains mapped until tbm_surface_unmap
        // below; we copy out before unmapping so the result is owned.
        let slice = unsafe { core::slice::from_raw_parts(plane.ptr, len) };
        slice.to_vec()
    };

    unsafe { tbm::tbm_surface_unmap(surface) };

    Ok(ScreenFrame {
        width: info.width,
        height: info.height,
        stride,
        bytes,
    })
}

/// Internal dispatch state. Captures the screenshooter + first wl_output,
/// plus the `shoot`/`area_shoot` completion flags.
#[derive(Default)]
struct State {
    shooter: Option<TizenScreenshooter>,
    shooter_version: Option<u32>,
    output: Option<WlOutput>,
    output_size: Option<(u32, u32)>,
    shot_done: bool,
    area_shot_done: bool,
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
            "tizen_screenshooter" => {
                let s = registry.bind::<TizenScreenshooter, _, _>(name, version, qh, ());
                state.shooter = Some(s);
                state.shooter_version = Some(version);
            }
            // Only capture the first output; ignore secondaries.
            "wl_output" if state.output.is_none() => {
                let o = registry.bind::<WlOutput, _, _>(name, version.min(2), qh, ());
                state.output = Some(o);
            }
            _ => {}
        }
    }
}

impl Dispatch<TizenScreenshooter, ()> for State {
    fn event(
        state: &mut Self,
        _: &TizenScreenshooter,
        event: ShooterEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ShooterEvent::Done => state.shot_done = true,
            ShooterEvent::AreaShootDone => state.area_shot_done = true,
            ShooterEvent::Format { .. } | ShooterEvent::ScreenshooterNotify { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for State {
    fn event(
        state: &mut Self,
        _: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_output::Event::Mode { width, height, .. } = event {
            if width > 0 && height > 0 {
                state.output_size = Some((width as u32, height as u32));
            }
        }
    }
}

impl Dispatch<WlBuffer, ()> for State {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        _: <WlBuffer as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// ---------------------------------------------------------------------------
// Raw `wl_display *` / `wl_event_queue *` extraction.
//
// `wayland-backend`'s `client_system` backend stores the underlying
// `wl_display *` and per-queue `wl_event_queue *` inside its FFI shim. We
// need both as raw pointers to hand to `libwayland-tbm-client`.

fn wl_display_ptr(conn: &Connection) -> *mut c_void {
    // `Connection::backend()` returns a cheaply-cloned `Backend` (Arc inside);
    // `Backend::display_ptr()` is only available under the `client_system`
    // feature, which we always enable in this crate's Cargo.toml.
    conn.backend().display_ptr() as *mut c_void
}

/// No-op user data for foreign-adopted proxies. `wl_buffer` only emits a
/// `release` event which we don't react to, so we don't need any state.
struct NoopData;
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

// Adopt a raw `wl_buffer *` (created by libwayland-tbm-client) into the
// wayland-client backend's object table so scanner-generated requests
// like `tizen_screenshooter.shoot(&output, &wl_buffer)` can reference it.
unsafe fn wl_buffer_from_ptr(conn: &Connection, ptr: *mut c_void) -> Result<WlBuffer> {
    let backend = conn.backend();
    let id =
        unsafe { backend.manage_object(WlBuffer::interface(), ptr as *mut _, Arc::new(NoopData)) };
    WlBuffer::from_id(conn, id)
        .map_err(|e| Error::BufferAllocation(format!("WlBuffer::from_id: {e:?}")))
}

/// On non-Tizen hosts, the dlopen path will fail at runtime. To keep
/// host `cargo check` clean without forcing a `cfg(tizen)` gate on every
/// caller, we additionally short-circuit `ScreenCapturer::new()` when
/// running on a non-Tizen host that has no compositor.
fn is_host_likely_tizen() -> bool {
    // Cheap proxy: the device has /usr/lib/libwayland-tbm-client.so.0.
    // On a dev workstation it doesn't. This is best-effort — the real
    // detection is wayland_tbm::is_available() at the FFI boundary.
    std::path::Path::new("/usr/lib/libwayland-tbm-client.so.0").exists()
        || std::path::Path::new("/usr/lib64/libwayland-tbm-client.so.0").exists()
}
