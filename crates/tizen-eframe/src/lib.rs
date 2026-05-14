//! eframe-style runner for egui applications on Tizen, backed by EGL/GLES.
//!
//! Implement [`App`] and pass it to [`run_native`] for the app-facing
//! path. [`TizenEguiGlow`] is the lower-level adapter for callers that
//! want to own the Tizen event loop.
//!
//! The public API mirrors `eframe`'s shape (`App`, `Frame`, `NativeOptions`,
//! `CreationContext`, `AppCreator`, `run_native`) so app code is portable
//! between `eframe` on desktop and this runner on Tizen — apps depend on
//! `tizen-eframe` (typically package-renamed to `eframe` in their
//! `Cargo.toml`) and write standard eframe boilerplate.

#![warn(missing_docs)]

use std::fmt;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use glow::HasContext;
use khronos_egl as egl;
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use tizen_egl::EglWindow;
pub use tizen_window::{keys, KeyGrab, KeyGrabMode, WindowType};
use tizen_window::{Display, Event, EventLoop, ModifiersState, MouseButton, Window, WindowBuilder};

/// Re-export of the egui crate used by this integration.
pub use egui;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the egui/Tizen integration.
#[derive(Debug)]
pub enum Error {
    /// Failure from `tizen-window`.
    Window(tizen_window::Error),
    /// Failure from `tizen-egl`.
    EglWindow(tizen_egl::Error),
    /// Could not read the display's raw window handle.
    DisplayHandle(raw_window_handle::HandleError),
    /// The display is not a Wayland display.
    NotWaylandDisplay,
    /// `eglGetDisplay` returned `NO_DISPLAY`.
    NoEglDisplay,
    /// No EGL config matched the requested GLES window surface.
    NoEglConfig,
    /// Failure from EGL.
    Egl(egl::Error),
    /// Failure while loading `libEGL`.
    EglLoad(String),
    /// Failure while creating the egui glow painter.
    Painter(String),
    /// Failure while constructing the application.
    AppCreation(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Window(e) => write!(f, "{e}"),
            Self::EglWindow(e) => write!(f, "{e}"),
            Self::DisplayHandle(e) => write!(f, "raw display handle: {e}"),
            Self::NotWaylandDisplay => f.write_str("display handle is not Wayland"),
            Self::NoEglDisplay => f.write_str("eglGetDisplay returned NO_DISPLAY"),
            Self::NoEglConfig => f.write_str("no matching EGL config"),
            Self::Egl(e) => write!(f, "{e}"),
            Self::EglLoad(e) => write!(f, "failed to load EGL: {e}"),
            Self::Painter(e) => write!(f, "egui_glow painter: {e}"),
            Self::AppCreation(e) => write!(f, "app creation failed: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<tizen_window::Error> for Error {
    fn from(value: tizen_window::Error) -> Self {
        Self::Window(value)
    }
}

impl From<tizen_egl::Error> for Error {
    fn from(value: tizen_egl::Error) -> Self {
        Self::EglWindow(value)
    }
}

impl From<raw_window_handle::HandleError> for Error {
    fn from(value: raw_window_handle::HandleError) -> Self {
        Self::DisplayHandle(value)
    }
}

impl From<egl::Error> for Error {
    fn from(value: egl::Error) -> Self {
        Self::Egl(value)
    }
}

/// Function used by [`run_native`] to construct the application after
/// the Tizen window, EGL context, and egui context are ready.
pub type AppCreator<'app> = Box<
    dyn 'app
        + FnOnce(
            &CreationContext,
        )
            -> std::result::Result<Box<dyn 'app + App>, Box<dyn std::error::Error + Send + Sync>>,
>;

/// Data passed to [`AppCreator`] while constructing an [`App`].
pub struct CreationContext {
    /// The egui context. Use this to configure fonts, visuals, style,
    /// texture loading, and other egui-global state before the first frame.
    pub egui_ctx: egui::Context,
    /// The glow context used by the renderer.
    pub gl: Option<Arc<glow::Context>>,
    /// Current window size in physical pixels.
    pub size: (u32, u32),
    /// Native physical pixels per egui point.
    pub pixels_per_point: f32,
}

/// App model mirroring `eframe::App`, backed by `tizen-window` +
/// `tizen-egl` + `egui_glow` instead of winit/glutin.
///
/// Apps that need the GL context for cleanup can clone the `Arc<glow::Context>`
/// from [`CreationContext::gl`] during construction and use it from `on_exit`.
pub trait App {
    /// Called each time the UI should repaint.
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame);

    /// Called once on shutdown after the final frame.
    fn on_exit(&mut self) {}

    /// Background clear colour used before egui paints.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::from_rgba_unmultiplied(12, 12, 12, 255).to_normalized_gamma_f32()
    }
}

/// Options for [`run_native`].
#[derive(Debug, Clone)]
pub struct NativeOptions {
    /// Window title.
    pub title: String,
    /// Reverse-DNS Tizen application id.
    pub app_id: String,
    /// Requested initial window size in physical pixels.
    pub size: (u32, u32),
    /// Native physical pixels per egui point.
    pub pixels_per_point: f32,
    /// Clear colour used before egui paints, as linear RGBA floats.
    pub clear_color: [f32; 4],
    /// Request a new frame after every paint.
    ///
    /// This is useful for demos and animation-heavy apps. Event-driven
    /// apps can keep this false and call `ctx.request_repaint()` or
    /// [`Frame::request_repaint`] when needed.
    pub continuous_repaint: bool,
    /// Tizen-policy window type. Use [`WindowType::Floating`] for
    /// chat/HUD/overlay apps — that's the dedicated Tizen request for
    /// a partial-window surface that can position over other
    /// toplevels. Defaults to [`WindowType::Toplevel`] (fullscreen).
    pub window_type: WindowType,
    /// Mark the surface as alpha-blended via
    /// `wl_surface.set_opaque_region(NULL)`. Pair with a
    /// transparent-looking `clear_color` (alpha = 0) to get the
    /// launcher / underlying app showing through the cleared regions.
    /// Defaults to `false`.
    pub transparent: bool,
    /// Ask the compositor not to take keyboard focus
    /// (`tizen_policy.set_focus_skip`). Right for passive overlays
    /// (notifications, chat tickers) that should sit on top of the
    /// launcher without stealing its remote-control input. Defaults
    /// to `false`.
    pub focus_skip: bool,
    /// IR-remote keycodes to grab via `tizen_keyrouter`. Without a
    /// grab, the keyrouter silently drops these keys even when this
    /// window is focused. Use the constants in [`keys`] for common
    /// TV remote codes. Defaults to empty.
    pub grab_keys: Vec<KeyGrab>,
    /// Automatically grab the Back key and call
    /// [`tizen_window::Window::set_should_close`] when it is pressed.
    /// Mode is auto-selected: [`KeyGrabMode::Topmost`] when the
    /// window can take focus, [`KeyGrabMode::Exclusive`] when
    /// [`Self::focus_skip`] is set so the overlay still exits on
    /// Back while the launcher keeps focus. Defaults to `true`.
    pub close_on_back: bool,
}

impl Default for NativeOptions {
    fn default() -> Self {
        Self {
            title: "tizen-eframe".to_owned(),
            app_id: "rust.tizen.eframe".to_owned(),
            size: (1920, 1080),
            pixels_per_point: 1.0,
            clear_color: [0.05, 0.05, 0.08, 1.0],
            continuous_repaint: false,
            window_type: WindowType::default(),
            transparent: false,
            focus_skip: false,
            grab_keys: Vec::new(),
            close_on_back: true,
        }
    }
}

/// Per-frame information passed to [`App::update`].
#[derive(Debug)]
pub struct Frame {
    frame_nr: u64,
    start_time: Instant,
    size: (u32, u32),
    repaint_requested: bool,
}

impl Frame {
    /// Number of frames painted since startup.
    pub fn frame_nr(&self) -> u64 {
        self.frame_nr
    }

    /// Elapsed time since the native runner started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Current window size in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Ask the runner to schedule another frame.
    pub fn request_repaint(&mut self) {
        self.repaint_requested = true;
    }
}

/// Result of painting one egui frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintResult {
    /// True if egui requested another repaint during this frame.
    pub repaint_requested: bool,
}

/// Run a complete native egui application on Tizen.
pub fn run_native(
    app_name: &str,
    mut options: NativeOptions,
    app_creator: AppCreator<'_>,
) -> Result<()> {
    if options.title.is_empty() {
        options.title = app_name.to_owned();
    }
    if options.app_id.is_empty() {
        options.app_id = app_name.to_owned();
    }

    let egui_options = TizenEguiOptions::from(&options);

    // Compose the keygrab list. The app's explicit `grab_keys` plus, if
    // `close_on_back`, an implicit Back grab so this exit path is
    // wired up by default. Mode for Back is auto-picked: Topmost when
    // the window can take focus, Exclusive when `focus_skip` is set
    // (so a launcher-keeps-focus overlay still exits on Back).
    let mut grab_keys: Vec<KeyGrab> = options.grab_keys.clone();
    if options.close_on_back && !grab_keys.iter().any(|g| g.name == keys::BACK) {
        let mode = if options.focus_skip {
            KeyGrabMode::Exclusive
        } else {
            KeyGrabMode::Topmost
        };
        grab_keys.push(KeyGrab {
            name: keys::BACK,
            mode,
        });
    }

    let mut display = Display::connect()?;
    // Resolve the BACK / ESCAPE names ahead of time so the
    // close_on_back loop can compare against raw keycodes without
    // doing an XKB lookup per key event.
    //
    // XKB keymaps use X11 keycodes (= Linux evdev + 8) for
    // historical reasons. `wl_keyboard.key.key`, by contrast,
    // delivers the raw kernel scancode (evdev). So to compare
    // resolved keysym keycodes against incoming wl events we
    // subtract 8 — see the comment block in
    // <https://wayland.app/protocols/wayland#wl_keyboard:event:key>.
    let back_keycodes: Vec<u32> = display
        .keycodes_for_name(keys::BACK)
        .into_iter()
        .chain(display.keycodes_for_name(keys::ESCAPE))
        .map(|x11_keycode| x11_keycode.saturating_sub(8))
        .collect();

    let mut window = WindowBuilder::new()
        .title(options.title.clone())
        .app_id(options.app_id.clone())
        .size(options.size.0, options.size.1)
        .window_type(options.window_type)
        .transparent(options.transparent)
        .focus_skip(options.focus_skip)
        .grab_keys(grab_keys)
        .build(&display)?;

    display.roundtrip(&mut window)?;

    // SAFETY: `run_native` owns `window` and `egui` in the same stack
    // frame. `egui` is created after `window` and destroyed before
    // `window` goes out of scope.
    let mut egui = unsafe { TizenEguiGlow::new(&display, &window, egui_options)? };
    let mut app = {
        let cc = CreationContext {
            egui_ctx: egui.context().clone(),
            gl: Some(Arc::clone(egui.gl_context())),
            size: window.size(),
            pixels_per_point: options.pixels_per_point,
        };
        app_creator(&cc).map_err(|e| Error::AppCreation(e.to_string()))?
    };

    let start_time = Instant::now();
    let mut frame_nr = 0_u64;
    window.request_redraw();

    // Calloop-based event loop. Consumes `display` (its wayland
    // connection + event queue are moved into the loop). Wayland
    // events drive `pending_events` on the window's state; SIGINT and
    // SIGTERM trip `should_close` via a calloop signal source so
    // Ctrl-C exits cleanly through the same shutdown path as a
    // compositor-initiated close.
    let event_loop = EventLoop::new(display)?;
    let continuous_repaint = options.continuous_repaint;
    let close_on_back = options.close_on_back;
    let mut window = event_loop.run(window, |window: &mut Window| -> Result<()> {
        let pending: Vec<Event> = window.drain_events().collect();
        for event in pending {
            // `close_on_back`: a remote-Back / keyboard-Escape press
            // trips the same shutdown path as Ctrl-C / a compositor
            // close. We pre-resolved the keycodes via the XKB keymap
            // at startup so this check is just a small slice contains.
            if close_on_back {
                // Fire on RELEASE, not press — matches the standard
                // TV / desktop UX where clicks / activations commit
                // when the key is let go (and prevents key-repeat
                // from firing multiple close requests).
                if let Event::KeyboardInput {
                    keycode,
                    pressed: false,
                    ..
                } = event
                {
                    if back_keycodes.contains(&keycode) {
                        window.set_should_close();
                    }
                }
            }

            let input_changed = egui.handle_event(&event);

            match event {
                Event::Resized { width, height } => {
                    egui.resize(width, height)?;
                    window.request_redraw();
                }
                Event::RedrawRequested => {
                    let visuals = egui.context().style().visuals.clone();
                    egui.set_clear_color(app.clear_color(&visuals));

                    let mut frame = Frame {
                        frame_nr,
                        start_time,
                        size: window.size(),
                        repaint_requested: false,
                    };
                    let paint_result = egui.run_and_paint(window, |ctx| {
                        app.update(ctx, &mut frame);
                    })?;
                    frame_nr += 1;

                    if continuous_repaint
                        || frame.repaint_requested
                        || paint_result.repaint_requested
                    {
                        window.request_redraw();
                    }
                }
                Event::CloseRequested => {}
                _ if input_changed => window.request_redraw(),
                _ => {}
            }
        }
        Ok(())
    })?;
    let _ = &mut window;

    app.on_exit();

    // We deliberately skip the synchronous Drop chain for `egui`
    // (TizenEguiGlow → EglState → EglWindow) and `window`
    // (WlSurface / xdg_toplevel proxies). On Tizen TV the Mesa
    // wayland-egl driver inside `eglDestroySurface` / `eglTerminate`
    // blocks waiting for `wl_buffer.release` events from the
    // compositor — but our wayland event queue lived inside the
    // `calloop` `EventLoop` that just exited, so nothing can service
    // those events any more and the process hangs forever in
    // `D (disk sleep)` state, unkillable until reboot.
    //
    // `std::process::exit(0)` short-circuits: the kernel closes our
    // wayland socket, the compositor observes a hard client
    // disconnect and tears down its side, and we exit immediately.
    // Standard mitigation pattern for Wayland-EGL clients whose
    // event-loop teardown can't keep dispatching during native
    // shutdown.
    //
    // A more graceful "post the teardown back into the event loop"
    // approach (mirroring winit's pattern, where the renderer
    // crate's EGL drops run while wayland dispatch is still alive)
    // is left as a future refactor — TODO.
    std::process::exit(0);
}

/// Options for constructing [`TizenEguiGlow`] directly.
#[derive(Debug, Clone)]
pub struct TizenEguiOptions {
    /// Native physical pixels per egui point.
    pub pixels_per_point: f32,
    /// Clear colour used before egui paints, as linear RGBA floats.
    pub clear_color: [f32; 4],
}

impl Default for TizenEguiOptions {
    fn default() -> Self {
        Self {
            pixels_per_point: 1.0,
            clear_color: [0.05, 0.05, 0.08, 1.0],
        }
    }
}

impl From<&NativeOptions> for TizenEguiOptions {
    fn from(value: &NativeOptions) -> Self {
        Self {
            pixels_per_point: value.pixels_per_point,
            clear_color: value.clear_color,
        }
    }
}

/// Low-level egui + glow adapter for a `tizen-window` window.
///
/// This owns the EGL surface/context, glow context, egui context, and
/// `egui_glow::Painter`. It does not own the Tizen `Window`; callers
/// using this type directly must keep the window alive until after this
/// adapter is destroyed.
pub struct TizenEguiGlow {
    egui_ctx: egui::Context,
    raw_input: egui::RawInput,
    pointer_pos: Option<egui::Pos2>,
    modifiers: egui::Modifiers,
    focused: bool,
    pixels_per_point: f32,
    clear_color: [f32; 4],
    repaint_requested: Arc<AtomicBool>,
    painter: egui_glow::Painter,
    gl: Arc<glow::Context>,
    egl: EglState,
    destroyed: bool,
}

impl TizenEguiGlow {
    /// Create the adapter for an existing Tizen display and window.
    ///
    /// # Safety
    ///
    /// The window's underlying `wl_surface` must outlive this adapter.
    /// Create this after the window, and destroy/drop it before the
    /// window is dropped.
    pub unsafe fn new(
        display: &Display,
        window: &Window,
        options: TizenEguiOptions,
    ) -> Result<Self> {
        let egl = unsafe { EglState::new(display, window)? };
        let gl = Arc::new(unsafe {
            glow::Context::from_loader_function(|name| {
                egl.lib
                    .get_proc_address(name)
                    .map(|p| p as *const _)
                    .unwrap_or(ptr::null())
            })
        });

        let painter = egui_glow::Painter::new(gl.clone(), "", None, false)
            .map_err(|e| Error::Painter(e.to_string()))?;
        let egui_ctx = egui::Context::default();
        egui_ctx.set_pixels_per_point(options.pixels_per_point);

        let repaint_requested = Arc::new(AtomicBool::new(false));
        let repaint_flag = Arc::clone(&repaint_requested);
        egui_ctx.set_request_repaint_callback(move |_| {
            repaint_flag.store(true, Ordering::Relaxed);
        });

        Ok(Self {
            egui_ctx,
            raw_input: egui::RawInput::default(),
            pointer_pos: None,
            modifiers: egui::Modifiers::default(),
            focused: true,
            pixels_per_point: options.pixels_per_point,
            clear_color: options.clear_color,
            repaint_requested,
            painter,
            gl,
            egl,
            destroyed: false,
        })
    }

    /// Access the egui context.
    pub fn context(&self) -> &egui::Context {
        &self.egui_ctx
    }

    /// Access the glow context used by the renderer.
    pub fn gl_context(&self) -> &Arc<glow::Context> {
        &self.gl
    }

    /// Set the background clear colour used before egui paints.
    pub fn set_clear_color(&mut self, clear_color: [f32; 4]) {
        self.clear_color = clear_color;
    }

    /// Convert a Tizen window event into egui input.
    ///
    /// Returns true when the event should trigger a repaint.
    pub fn handle_event(&mut self, event: &Event) -> bool {
        match *event {
            Event::Focused(focused) => {
                self.focused = focused;
                self.raw_input
                    .events
                    .push(egui::Event::WindowFocused(focused));
                true
            }
            Event::CursorEntered { x, y } | Event::CursorMoved { x, y } => {
                let pos = self.to_egui_pos(x, y);
                self.pointer_pos = Some(pos);
                self.raw_input.events.push(egui::Event::PointerMoved(pos));
                true
            }
            Event::CursorLeft => {
                self.pointer_pos = None;
                self.raw_input.events.push(egui::Event::PointerGone);
                true
            }
            Event::MouseInput { button, pressed } => {
                if let (Some(pos), Some(button)) = (self.pointer_pos, map_mouse_button(button)) {
                    self.raw_input.events.push(egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers: self.modifiers,
                    });
                    true
                } else {
                    false
                }
            }
            Event::MouseWheel { dx, dy } => {
                self.raw_input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(dx as f32, dy as f32),
                    modifiers: self.modifiers,
                });
                true
            }
            Event::KeyboardInput {
                keycode,
                pressed,
                modifiers,
            } => {
                self.modifiers = map_modifiers(modifiers);
                self.raw_input.modifiers = self.modifiers;
                if let Some(key) = map_key(keycode) {
                    self.raw_input.events.push(egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed,
                        repeat: false,
                        modifiers: self.modifiers,
                    });
                    true
                } else {
                    false
                }
            }
            Event::Resized { .. } | Event::RedrawRequested | Event::CloseRequested => false,
            _ => false,
        }
    }

    /// Resize the underlying EGL window.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.egl.egl_window.resize(width, height, 0, 0)?;
        Ok(())
    }

    /// Run one egui pass and paint it to the current EGL surface.
    pub fn run_and_paint<F>(&mut self, window: &Window, run_ui: F) -> Result<PaintResult>
    where
        F: FnMut(&egui::Context),
    {
        self.repaint_requested.store(false, Ordering::Relaxed);

        let (width, height) = window.size();
        self.raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(
                width as f32 / self.pixels_per_point,
                height as f32 / self.pixels_per_point,
            ),
        ));
        self.raw_input.time = Some(self.egl.start.elapsed().as_secs_f64());
        self.raw_input.focused = self.focused;
        self.raw_input.modifiers = self.modifiers;

        let raw_input = self.raw_input.take();
        let full_output = self.egui_ctx.run(raw_input, run_ui);
        let primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        unsafe {
            self.gl.viewport(0, 0, width as i32, height as i32);
            self.gl.clear_color(
                self.clear_color[0],
                self.clear_color[1],
                self.clear_color[2],
                self.clear_color[3],
            );
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }

        self.painter.paint_and_update_textures(
            [width, height],
            full_output.pixels_per_point,
            &primitives,
            &full_output.textures_delta,
        );
        self.egl
            .lib
            .swap_buffers(self.egl.display, self.egl.surface)?;

        Ok(PaintResult {
            repaint_requested: self.repaint_requested.swap(false, Ordering::Relaxed),
        })
    }

    /// Explicitly destroy GL resources owned by the egui painter.
    pub fn destroy(&mut self) {
        if !self.destroyed {
            self.painter.destroy();
            self.destroyed = true;
        }
    }

    fn to_egui_pos(&self, x: f64, y: f64) -> egui::Pos2 {
        egui::pos2(
            x as f32 / self.pixels_per_point,
            y as f32 / self.pixels_per_point,
        )
    }
}

impl Drop for TizenEguiGlow {
    fn drop(&mut self) {
        self.destroy();
    }
}

struct EglState {
    lib: egl::DynamicInstance<egl::EGL1_4>,
    display: egl::Display,
    context: egl::Context,
    surface: egl::Surface,
    egl_window: EglWindow,
    start: Instant,
}

impl EglState {
    unsafe fn new(display: &Display, window: &Window) -> Result<Self> {
        let (width, height) = window.size();
        let egl_window = unsafe { EglWindow::new(window, width, height)? };
        let lib = unsafe {
            egl::DynamicInstance::<egl::EGL1_4>::load_required()
                .map_err(|e| Error::EglLoad(e.to_string()))?
        };

        let wl_display_ptr = match display.display_handle()?.as_raw() {
            RawDisplayHandle::Wayland(wl) => wl.display.as_ptr(),
            _ => return Err(Error::NotWaylandDisplay),
        };
        let egl_display = unsafe { lib.get_display(wl_display_ptr).ok_or(Error::NoEglDisplay)? };
        lib.initialize(egl_display)?;
        lib.bind_api(egl::OPENGL_ES_API)?;

        let config = choose_config(&lib, egl_display)?;
        let context = match create_context(&lib, egl_display, config, 3) {
            Ok(context) => context,
            Err(_) => create_context(&lib, egl_display, config, 2)?,
        };
        let surface =
            unsafe { lib.create_window_surface(egl_display, config, egl_window.as_ptr(), None)? };
        lib.make_current(egl_display, Some(surface), Some(surface), Some(context))?;

        Ok(Self {
            lib,
            display: egl_display,
            context,
            surface,
            egl_window,
            start: Instant::now(),
        })
    }
}

impl Drop for EglState {
    fn drop(&mut self) {
        // `run_native` short-circuits via `process::exit(0)` before
        // dropping us — see the rationale block there. The body below
        // therefore runs only for direct `TizenEguiGlow` users (i.e.
        // someone using the low-level adapter without `run_native`),
        // and intentionally skips the calls that hang on Tizen TV:
        //   - `eglDestroySurface` blocks waiting for compositor
        //     `wl_buffer.release` events,
        //   - `eglTerminate` likewise blocks.
        // Both are released by the OS at process exit, matching
        // glutin's documented design choice for skipping
        // `eglTerminate` on drop (see `glutin/src/api/egl/display.rs`).
        let _ = self.lib.make_current(self.display, None, None, None);
        let _ = self.lib.destroy_context(self.display, self.context);
    }
}

fn choose_config(
    lib: &egl::DynamicInstance<egl::EGL1_4>,
    display: egl::Display,
) -> Result<egl::Config> {
    // Request an alpha channel so apps that paint with transparent pixels
    // (e.g. `clear_color = [_, _, _, 0.0]`) get a real alpha-blended
    // surface that the compositor can composite over what's beneath.
    //
    // `eglChooseConfig`'s sort order prefers smaller total color-buffer
    // bits, which on some drivers means an XRGB (alpha=0) config wins
    // over an ARGB (alpha=8) one even when we request ALPHA_SIZE=8.
    // So we enumerate the matching configs and pick the first with
    // ALPHA_SIZE >= 8 ourselves.
    let attrs = [
        egl::SURFACE_TYPE,
        egl::WINDOW_BIT,
        egl::RED_SIZE,
        8,
        egl::GREEN_SIZE,
        8,
        egl::BLUE_SIZE,
        8,
        egl::ALPHA_SIZE,
        8,
        egl::DEPTH_SIZE,
        0,
        egl::STENCIL_SIZE,
        0,
        egl::RENDERABLE_TYPE,
        egl::OPENGL_ES2_BIT,
        egl::NONE,
    ];

    let mut configs: Vec<egl::Config> = Vec::with_capacity(32);
    lib.choose_config(display, &attrs, &mut configs)?;
    for cfg in &configs {
        if lib.get_config_attrib(display, *cfg, egl::ALPHA_SIZE)? >= 8 {
            return Ok(*cfg);
        }
    }
    Err(Error::NoEglConfig)
}

fn create_context(
    lib: &egl::DynamicInstance<egl::EGL1_4>,
    display: egl::Display,
    config: egl::Config,
    client_version: i32,
) -> Result<egl::Context> {
    let attrs = [egl::CONTEXT_CLIENT_VERSION, client_version, egl::NONE];
    Ok(lib.create_context(display, config, None, &attrs)?)
}

fn map_mouse_button(button: MouseButton) -> Option<egui::PointerButton> {
    match button {
        MouseButton::Left => Some(egui::PointerButton::Primary),
        MouseButton::Right => Some(egui::PointerButton::Secondary),
        MouseButton::Middle => Some(egui::PointerButton::Middle),
        MouseButton::Back => Some(egui::PointerButton::Extra1),
        MouseButton::Forward => Some(egui::PointerButton::Extra2),
        MouseButton::Other(_) => None,
        _ => None,
    }
}

fn map_modifiers(modifiers: ModifiersState) -> egui::Modifiers {
    let ctrl = modifiers.contains(ModifiersState::CTRL);
    egui::Modifiers {
        alt: modifiers.contains(ModifiersState::ALT),
        ctrl,
        shift: modifiers.contains(ModifiersState::SHIFT),
        mac_cmd: false,
        command: ctrl,
    }
}

fn map_key(keycode: u32) -> Option<egui::Key> {
    match keycode {
        1 | 158 => Some(egui::Key::Escape),
        14 => Some(egui::Key::Backspace),
        15 => Some(egui::Key::Tab),
        28 => Some(egui::Key::Enter),
        57 => Some(egui::Key::Space),
        102 => Some(egui::Key::Home),
        103 => Some(egui::Key::ArrowUp),
        104 => Some(egui::Key::PageUp),
        105 => Some(egui::Key::ArrowLeft),
        106 => Some(egui::Key::ArrowRight),
        107 => Some(egui::Key::End),
        108 => Some(egui::Key::ArrowDown),
        109 => Some(egui::Key::PageDown),
        110 => Some(egui::Key::Insert),
        111 => Some(egui::Key::Delete),
        _ => None,
    }
}
