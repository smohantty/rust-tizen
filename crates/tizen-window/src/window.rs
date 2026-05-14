use std::ffi::c_void;
use std::os::raw::c_int;
use std::ptr::NonNull;

use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, WaylandWindowHandle, WindowHandle,
};
use tizen_tbm_sys::tbm;
use tizen_tbm_sys::wayland_tbm;
use tizen_window_sys::tizen_extension::tizen_policy::TizenPolicy;
use tizen_window_sys::wtz_shell::{wtz_shell::WtzShell, wtz_surface::WtzSurface};
use tizen_window_sys::xdg_shell_v6::{
    zxdg_shell_v6::ZxdgShellV6, zxdg_surface_v6::ZxdgSurfaceV6, zxdg_toplevel_v6::ZxdgToplevelV6,
};
use wayland_client::protocol::{
    wl_buffer::WlBuffer, wl_compositor::WlCompositor, wl_keyboard::WlKeyboard,
    wl_pointer::WlPointer, wl_seat::WlSeat, wl_surface::WlSurface,
};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use crate::display::{adopt_wl_buffer, Display};
use crate::error::{Error, Result};

/// Tizen window type, applied via `tizen_policy.set_type`. The choice
/// affects z-order, focus policy, and on some Tizen compositors whether
/// the surface is forced fullscreen or allowed to remain at its
/// requested size with alpha blending.
///
/// Values mirror the `win_type` enum from `tizen-extension.xml`.
/// `Toplevel` is the default and matches a normal application window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowType {
    /// Normal application window (default). Tizen compositors typically
    /// force this fullscreen and treat it as opaque.
    #[default]
    Toplevel,
    /// **Floating** — a partial window that can position over other
    /// top-level windows. This is the right choice for chat/HUD/overlay
    /// apps that want a non-fullscreen surface. Implemented as
    /// `tizen_policy.set_floating_mode` plus `set_type(NONE)`, matching
    /// `tizen-core-wayland`'s `WINDOW_TYPE_FLOATING` mapping.
    Floating,
    /// Fullscreen state.
    Fullscreen,
    /// Maximized state.
    Maximized,
    /// Transient relation state.
    Transient,
    /// Menu.
    Menu,
    /// Custom (no policy enforced by the compositor).
    Custom,
    /// Notification — z-ordered above toplevels.
    Notification,
    /// Auxiliary "utility" window.
    Utility,
    /// Dialog.
    Dialog,
    /// Dock.
    Dock,
    /// Splash screen.
    Splash,
}

impl WindowType {
    fn as_u32(self) -> u32 {
        // Values from `protocols/tizen-extension.xml` enum win_type.
        // `Floating` maps to `NONE` (0) because it's set via the
        // separate `set_floating_mode` request, not `set_type`.
        match self {
            Self::Floating => 0,
            Self::Toplevel => 1,
            Self::Fullscreen => 2,
            Self::Maximized => 3,
            Self::Transient => 4,
            Self::Menu => 5,
            Self::Custom => 7,
            Self::Notification => 8,
            Self::Utility => 9,
            Self::Dialog => 10,
            Self::Dock => 11,
            Self::Splash => 12,
        }
    }
}

/// Builder for [`Window`]. Created via [`WindowBuilder::new`].
#[derive(Debug, Clone, Default)]
pub struct WindowBuilder {
    title: Option<String>,
    app_id: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    window_type: WindowType,
    transparent: bool,
}

impl WindowBuilder {
    /// Start a builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Window title (shown in launcher / overview).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Reverse-DNS app id (`org.tizen.example`). The compositor uses
    /// this for window grouping and launcher icons.
    pub fn app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = Some(app_id.into());
        self
    }

    /// Requested initial size. The compositor may override this in
    /// its first `configure` event — call [`Window::size`] after
    /// `Display::roundtrip` to get the real size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Tizen-policy [`WindowType`] applied via `tizen_policy.set_type`
    /// (plus `set_floating_mode` for [`WindowType::Floating`]).
    /// Affects z-order, focus, and on some compositors whether the
    /// surface gets forced fullscreen.
    /// Defaults to [`WindowType::Toplevel`].
    pub fn window_type(mut self, window_type: WindowType) -> Self {
        self.window_type = window_type;
        self
    }

    /// Mark the surface as alpha-blended ("not opaque") via
    /// `wl_surface.set_opaque_region(NULL)`. Required for the
    /// compositor to actually composite RGBA pixels over what's
    /// underneath — without it, even an EGL surface with ALPHA_SIZE=8
    /// is treated as opaque and rendered as solid black where alpha=0.
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Create the window — sends all the protocol requests and
    /// returns once the surface is committed and ready to render.
    pub fn build(self, display: &Display) -> Result<Window> {
        let qh = display.queue_handle();
        let surface = display.compositor.create_surface(&qh, ());
        let xdg_surface = display.xdg_shell.get_xdg_surface(&surface, &qh, ());
        let xdg_toplevel = xdg_surface.get_toplevel(&qh, ());

        if let Some(t) = &self.title {
            xdg_toplevel.set_title(t.clone());
        }
        if let Some(a) = &self.app_id {
            xdg_toplevel.set_app_id(a.clone());
        }

        let wtz_surface = display.wtz_shell.get_wtz_surface(&surface, &qh, ());

        // Mark the surface as alpha-blended before the first commit so
        // the compositor honours RGBA pixels (without this, even an
        // EGL surface with ALPHA_SIZE=8 is treated as opaque on Tizen).
        // Mirrors `tizen_core_wl_window_set_alpha(window, true)` in
        // `tizen-core-wayland/.../tizen_core_wl_surface.c:2181`.
        if self.transparent {
            surface.set_opaque_region(None);
        }

        // First commit — required by xdg_shell before the compositor
        // will send the initial `configure` event.
        surface.commit();

        // Tizen-specific visibility + type policy. Without these
        // requests the compositor lays out our surface but keeps it
        // BEHIND the launcher / system UI. `tizen_core_wl`'s
        // `tizen_core_wl_window_show()` (tizen_core_wl_surface.c:2111)
        // does:
        //
        //   tizen_policy.set_type(surface, win_type)
        //   tizen_policy.show(surface)         // since v8
        //
        // For `Floating`, we additionally call `set_floating_mode`
        // (mirroring `tizen_core_wl_window_set_type` in
        // `tizen_core_wl_surface.c:1498` for `WINDOW_TYPE_FLOATING`)
        // — this is the request that tells the compositor the
        // surface is a partial window that can sit over toplevels
        // rather than being forced fullscreen.
        //
        // We intentionally do NOT call `activate` here — `activate`
        // grabs keyboard focus, and on a TV target that means the
        // launcher's IR-remote events stop being delivered to the
        // launcher and start going to us. If our process then dies
        // (or doesn't handle keys), the remote becomes unresponsive
        // until the compositor times out our focus or reboots.
        // `raise` is included because it just bumps z-order — no
        // focus side effect.
        //
        // Doing this in `build()` (after the empty xdg-shell commit but
        // before any content buffer is attached) means both the TBM
        // and EGL render paths get visibility set up identically — the
        // EGL path never calls our internal `paint()`.
        if let Some(tp) = &display.tz_policy {
            if self.window_type == WindowType::Floating {
                tp.set_floating_mode(&surface);
            }
            tp.set_type(&surface, self.window_type.as_u32());
            tp.show(&surface);
            tp.raise(&surface);
        }

        Ok(Window {
            state: WindowState {
                width: self.width.unwrap_or(640),
                height: self.height.unwrap_or(480),
                pending_configure: None,
                configured: false,
                should_close: false,
                pixel: 0xFF1E40AFu32,
                ..Default::default()
            },
            surface,
            xdg_surface,
            xdg_toplevel,
            wtz_surface,
            tbm_client_ptr: display.tbm_client.ptr,
            conn: display.conn.clone(),
            qh,
        })
    }
}

/// A native Tizen Wayland window. Holds the wl_surface +
/// xdg/wtz roles + buffer; drop it to release them.
pub struct Window {
    pub(crate) state: WindowState,
    pub(crate) surface: WlSurface,
    // These three are held purely for their `Drop` side effect — when
    // the Window is dropped, scanner-generated destructors fire and
    // tell the compositor to release the corresponding object.
    #[allow(dead_code)]
    pub(crate) xdg_surface: ZxdgSurfaceV6,
    #[allow(dead_code)]
    pub(crate) xdg_toplevel: ZxdgToplevelV6,
    #[allow(dead_code)]
    pub(crate) wtz_surface: WtzSurface,
    pub(crate) tbm_client_ptr: *mut wayland_tbm::wayland_tbm_client,
    pub(crate) conn: Connection,
    /// Queue handle for this window's surface — needed to register
    /// `wl_surface.frame` callbacks on the right event queue.
    pub(crate) qh: QueueHandle<WindowState>,
}

impl Window {
    /// Current pixel size — updated by the compositor's `configure`.
    pub fn size(&self) -> (u32, u32) {
        (self.state.width, self.state.height)
    }

    /// Has the compositor told us to close (X button, system kill,
    /// etc.)?
    pub fn should_close(&self) -> bool {
        self.state.should_close
    }

    /// Schedule a redraw at the compositor's next available frame
    /// (one-shot — call again from the redraw handler to keep
    /// looping). Registers a `wl_surface.frame` callback; the
    /// compositor fires it at ~vsync pace via [`Event::RedrawRequested`].
    ///
    /// This is the only correct way to do animated rendering on
    /// Wayland — busy-looping `commit()` calls would just queue
    /// frames the compositor drops.
    pub fn request_redraw(&mut self) {
        // The callback is registered on our queue (qh); the
        // scanner-generated `frame` returns a `wl_callback` proxy
        // whose `done` event flips `state.frame_requested = false`
        // and pushes `Event::RedrawRequested`. We don't hold onto the
        // callback handle — it auto-destroys on `done`.
        let _cb = self.surface.frame(&self.qh, ());
        self.state.frame_requested = true;
    }

    /// Drain any events the dispatcher has queued for the caller.
    /// Returns each pending event in arrival order. Empty when
    /// nothing has happened since the last drain.
    ///
    /// Most callers use [`Display::run`](crate::Display::run) instead,
    /// which drains and dispatches on each loop iteration. This is the
    /// escape hatch for callers running their own dispatch loop.
    pub fn drain_events(&mut self) -> impl Iterator<Item = crate::Event> + '_ {
        self.state.pending_events.drain(..)
    }

    /// Set the solid colour to paint on the next configure / redraw.
    /// Bytes are interpreted as **BGRX** in memory (the native
    /// XRGB8888 layout TBM allocates) — i.e. `0x00_BB_GG_RR`.
    pub fn fill_solid(&mut self, bgrx: u32) {
        self.state.pixel = bgrx;
        // If we're already configured, render immediately; otherwise
        // the first configure handler will pick this up.
        if self.state.configured {
            let _ = self.paint();
        }
    }

    /// Allocate a TBM buffer of the current size, fill with
    /// `state.pixel`, attach + damage + commit.
    pub(crate) fn paint(&mut self) -> Result<()> {
        let w = self.state.width as c_int;
        let h = self.state.height as c_int;
        // SAFETY: `tbm_surface_create` returns a valid handle or NULL;
        // we null-check below.
        let tbm = unsafe { tbm::tbm_surface_create(w, h, tbm::TBM_FORMAT_XRGB8888) };
        if tbm.is_null() {
            return Err(Error::Tbm(format!("tbm_surface_create({w}, {h}) NULL")));
        }

        // Fill the TBM buffer with the solid colour. Map for WRITE.
        let mut info = tbm::tbm_surface_info_s::default();
        let rc = unsafe { tbm::tbm_surface_map(tbm, tbm::TBM_SURF_OPTION_WRITE, &mut info) };
        if rc != 0 {
            unsafe { tbm::tbm_surface_destroy(tbm) };
            return Err(Error::Tbm(format!("tbm_surface_map rc={rc}")));
        }
        let stride = info.planes[0].stride;
        let ptr = info.planes[0].ptr;
        if !ptr.is_null() {
            // SAFETY: `ptr` is mapped writable for `stride * height` bytes
            // by tbm_surface_map.
            unsafe {
                let pixel = self.state.pixel;
                for row in 0..info.height {
                    let row_ptr = ptr.add((row * stride) as usize) as *mut u32;
                    for col in 0..info.width {
                        *row_ptr.add(col as usize) = pixel;
                    }
                }
            }
        }
        unsafe { tbm::tbm_surface_unmap(tbm) };

        // Wrap as wl_buffer.
        let wl_buf_ptr = unsafe {
            wayland_tbm::create_buffer(self.tbm_client_ptr, tbm)
                .map_err(|e| Error::Tbm(e.to_string()))?
        };
        if wl_buf_ptr.is_null() {
            unsafe { tbm::tbm_surface_destroy(tbm) };
            return Err(Error::Tbm("wayland_tbm_client_create_buffer NULL".into()));
        }
        let wl_buffer = unsafe { adopt_wl_buffer(&self.conn, wl_buf_ptr)? };

        self.surface.attach(Some(&wl_buffer), 0, 0);
        self.surface.damage_buffer(0, 0, w, h);
        self.surface.commit();

        // Visibility policy (`tz_policy.set_type/show/raise`) is applied
        // once in `WindowBuilder::build` so both TBM and EGL render
        // paths share it.

        // We deliberately leak the TBM surface + wl_buffer here: the
        // compositor still owns them until it sends `wl_buffer.release`.
        // A real production crate would track them and free on release.
        // For the MVP, we'll just allocate fresh each paint — the
        // surface is mostly static.
        let _ = (wl_buffer, tbm);

        Ok(())
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // wayland-scanner generates destructor requests; dropping the
        // proxies sends them.
    }
}

// ----- raw-window-handle interop -------------------------------------------
//
// `HasWindowHandle` lets any renderer (softbuffer, glow via khronos-egl,
// wgpu's GLES backend, ash for Vulkan, …) discover our underlying
// `wl_surface *` without us taking a dependency on those crates. This is
// the standard ecosystem trait (see `raw-window-handle` 0.6).

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        // The scanner-generated `WlSurface` exposes `.id().as_ptr()` which
        // returns the underlying `*mut wl_proxy`. For a `wl_surface` the
        // proxy *is* the surface from libwayland-client's perspective.
        let ptr = self.surface.id().as_ptr() as *mut c_void;
        let nn = NonNull::new(ptr).ok_or(HandleError::Unavailable)?;
        let raw = WaylandWindowHandle::new(nn);
        // SAFETY: the `wl_surface *` is valid for the lifetime of `self.surface`,
        // which is owned by this `Window`; `WindowHandle::borrow_raw` ties the
        // returned handle's lifetime to `&self`.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Wayland(raw)) })
    }
}

// ---------------------------------------------------------------------------
// Dispatch state — captures globals during registry walk and tracks
// the configure handshake / close flag.

#[derive(Default)]
pub(crate) struct WindowState {
    // Globals captured during registry walk (also held by Display).
    pub(crate) compositor: Option<WlCompositor>,
    pub(crate) xdg_shell: Option<ZxdgShellV6>,
    pub(crate) wtz_shell: Option<WtzShell>,
    pub(crate) tz_policy: Option<TizenPolicy>,
    pub(crate) seat: Option<WlSeat>,
    /// Bound lazily when the seat advertises the `pointer` capability.
    /// Held for the duration of the queue — dropped via the
    /// scanner-generated destructor when [`Display`] drops.
    pub(crate) pointer: Option<WlPointer>,
    /// Bound lazily when the seat advertises the `keyboard` capability.
    /// Same lifecycle as [`Self::pointer`].
    pub(crate) keyboard: Option<WlKeyboard>,

    // Window-level state.
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Pending serial from the latest `zxdg_surface_v6.configure` — we
    /// ack it on the next commit.
    pub(crate) pending_configure: Option<u32>,
    /// Set once we've received and acked the first configure. The
    /// first paint is gated on this.
    pub(crate) configured: bool,
    /// Compositor asked us to close.
    pub(crate) should_close: bool,
    /// BGRX value to paint on (re)configure.
    pub(crate) pixel: u32,

    /// True between a `Window::request_redraw()` call and the matching
    /// `wl_callback.done` event. Lets us skip duplicate frame
    /// registrations and tell whether a stale `RedrawRequested` is
    /// pending.
    pub(crate) frame_requested: bool,

    /// Events the dispatcher has produced since the last drain. The
    /// `Window::drain_events` method pulls these out for the caller.
    pub(crate) pending_events: Vec<crate::Event>,
}

impl Dispatch<WlSurface, ()> for WindowState {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: <WlSurface as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, ()> for WindowState {
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

impl Dispatch<ZxdgShellV6, ()> for WindowState {
    fn event(
        _: &mut Self,
        proxy: &ZxdgShellV6,
        event: <ZxdgShellV6 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // `ping` event must be ack'd or the compositor kills the
        // connection.
        use tizen_window_sys::xdg_shell_v6::zxdg_shell_v6::Event;
        if let Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<ZxdgSurfaceV6, ()> for WindowState {
    fn event(
        state: &mut Self,
        proxy: &ZxdgSurfaceV6,
        event: <ZxdgSurfaceV6 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use tizen_window_sys::xdg_shell_v6::zxdg_surface_v6::Event;
        if let Event::Configure { serial } = event {
            // Don't ack here — ack with the next commit so the compositor
            // sees the size we actually rendered. Stash the serial.
            state.pending_configure = Some(serial);
            // Immediately ack + commit a paint to satisfy the v6
            // contract. The first paint uses the size from the toplevel
            // configure event (already stashed) or our default.
            proxy.ack_configure(serial);
            state.pending_configure = None;
            let was_configured = state.configured;
            state.configured = true;
            // First-ever configure: tell the user it's time to paint.
            // (Subsequent configures get their RedrawRequested via the
            // Resized path in zxdg_toplevel_v6 below.)
            if !was_configured {
                state.pending_events.push(crate::Event::RedrawRequested);
            }
        }
    }
}

impl Dispatch<ZxdgToplevelV6, ()> for WindowState {
    fn event(
        state: &mut Self,
        _: &ZxdgToplevelV6,
        event: <ZxdgToplevelV6 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use tizen_window_sys::xdg_shell_v6::zxdg_toplevel_v6::Event;
        match event {
            Event::Configure { width, height, .. } if width > 0 && height > 0 => {
                let (w, h) = (width as u32, height as u32);
                if state.width != w || state.height != h {
                    state.width = w;
                    state.height = h;
                    state.pending_events.push(crate::Event::Resized {
                        width: w,
                        height: h,
                    });
                    // Pair Resized with RedrawRequested so the caller's
                    // standard "draw on RedrawRequested" handler also
                    // handles resizes.
                    state.pending_events.push(crate::Event::RedrawRequested);
                }
            }
            Event::Close => {
                state.should_close = true;
                state.pending_events.push(crate::Event::CloseRequested);
            }
            _ => {}
        }
    }
}

// The `wl_callback` proxy returned by `wl_surface.frame()`. We use
// `()` user-data and assume all `wl_callback.done` events on our queue
// are frame callbacks — fine because we don't use `wl_display.sync`
// or other callback-returning requests on this queue.
impl Dispatch<wayland_client::protocol::wl_callback::WlCallback, ()> for WindowState {
    fn event(
        state: &mut Self,
        _: &wayland_client::protocol::wl_callback::WlCallback,
        event: wayland_client::protocol::wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // `done` is the only event the callback emits; it carries a
        // timestamp we don't currently surface (Phase 1.2: just a
        // RedrawRequested signal).
        if let wayland_client::protocol::wl_callback::Event::Done { .. } = event {
            state.frame_requested = false;
            state.pending_events.push(crate::Event::RedrawRequested);
        }
    }
}

impl Dispatch<WtzShell, ()> for WindowState {
    fn event(
        _: &mut Self,
        _: &WtzShell,
        _: <WtzShell as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WtzSurface, ()> for WindowState {
    fn event(
        _: &mut Self,
        _: &WtzSurface,
        _: <WtzSurface as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Decoration + screen events; we don't act on them in the MVP.
    }
}

impl Dispatch<TizenPolicy, ()> for WindowState {
    fn event(
        _: &mut Self,
        _: &TizenPolicy,
        _: <TizenPolicy as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // tizen_policy emits notifications for conformant area changes,
        // notification-window done, etc. None of those are relevant to
        // making a basic window visible — silently ignore.
    }
}

// SAFETY: tbm_client_ptr is an opaque handle from libwayland-tbm-client.so
// that we treat as Send. We never share mutably across threads — the
// Window itself is not Sync.
unsafe impl Send for Window {}

// Compatibility: c_void to silence unused-import in some configs.
#[doc(hidden)]
type _Cv = c_void;
