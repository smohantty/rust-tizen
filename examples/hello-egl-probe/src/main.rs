//! Go/no-go gate for GLES 3.x on Tizen.
//!
//! Opens a window via `tizen-window`, builds a `wl_egl_window` via
//! `tizen-egl`, initialises EGL via `khronos-egl`, asks for a GLES 3
//! context (falling back to 2), and prints what the driver advertises:
//!
//! ```text
//! EGL_VENDOR        / EGL_CLIENT_APIS / EGL_EXTENSIONS
//! GL_VENDOR         / GL_RENDERER     / GL_VERSION / GL_SHADING_LANGUAGE_VERSION
//! ```
//!
//! If `GL_VERSION` says `OpenGL ES 3.x`, egui_glow is on the table. If
//! only `OpenGL ES 2.x` comes back, Phase 3 will need an older egui.
//!
//! ## Cross-compile + run
//!
//! ```sh
//! cd examples/hello-egl-probe
//! cargo tizen build --release
//! # push the binary to the device, then on-device:
//! XDG_RUNTIME_DIR=/run WAYLAND_DISPLAY=wayland-0 /tmp/hello-egl-probe
//! ```

use std::ptr;

use khronos_egl as egl;
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use tizen::egl::EglWindow;
use tizen::window::{Display, WindowBuilder};

fn main() -> std::process::ExitCode {
    if let Err(e) = run() {
        eprintln!("hello-egl-probe: {e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut display = Display::connect()?;
    let mut window = WindowBuilder::new()
        .title("hello-egl-probe")
        .app_id("rust.tizen.hello-egl-probe")
        .size(640, 480)
        .build(&display)?;
    display.roundtrip(&mut window)?;
    let (w, h) = window.size();
    println!("hello-egl-probe: window {w}x{h}");

    // SAFETY: `window` is kept alive for the rest of `run`, which is
    // strictly longer than `egl_window`.
    let egl_window = unsafe { EglWindow::new(&window, w, h)? };

    // Load libEGL.so.1 from the device.
    let egl_lib = unsafe { egl::DynamicInstance::<egl::EGL1_4>::load_required()? };

    // Use the wayland wl_display * that tizen-window already holds.
    let display_handle = display
        .display_handle()
        .map_err(|e| format!("display_handle: {e}"))?;
    let wl_display_ptr = match display_handle.as_raw() {
        RawDisplayHandle::Wayland(wl) => wl.display.as_ptr(),
        _ => return Err("display handle is not Wayland".into()),
    };

    let egl_display = unsafe {
        egl_lib.get_display(wl_display_ptr).ok_or("eglGetDisplay returned NO_DISPLAY")?
    };
    let (major, minor) = egl_lib.initialize(egl_display)?;
    println!("hello-egl-probe: EGL_VERSION {major}.{minor}");

    let vendor = egl_lib.query_string(Some(egl_display), egl::VENDOR)?;
    let apis = egl_lib.query_string(Some(egl_display), egl::CLIENT_APIS)?;
    let ext = egl_lib.query_string(Some(egl_display), egl::EXTENSIONS)?;
    println!("hello-egl-probe: EGL_VENDOR       = {}", vendor.to_string_lossy());
    println!("hello-egl-probe: EGL_CLIENT_APIS  = {}", apis.to_string_lossy());
    println!("hello-egl-probe: EGL_EXTENSIONS   = {}", ext.to_string_lossy());

    egl_lib.bind_api(egl::OPENGL_ES_API)?;

    // Configs.
    let cfg_attrs = [
        egl::SURFACE_TYPE, egl::WINDOW_BIT,
        egl::RED_SIZE, 8,
        egl::GREEN_SIZE, 8,
        egl::BLUE_SIZE, 8,
        egl::ALPHA_SIZE, 0,
        egl::DEPTH_SIZE, 0,
        egl::STENCIL_SIZE, 0,
        egl::RENDERABLE_TYPE, egl::OPENGL_ES2_BIT,
        egl::NONE,
    ];
    let config = egl_lib
        .choose_first_config(egl_display, &cfg_attrs)?
        .ok_or("no matching EGL config")?;

    // Try GLES 3 first, then 2.
    let (context, gl_major) = match try_context(&egl_lib, egl_display, config, 3) {
        Ok(c) => (c, 3),
        Err(e3) => {
            println!("hello-egl-probe: GLES 3 context failed ({e3}), trying GLES 2");
            (try_context(&egl_lib, egl_display, config, 2)?, 2)
        }
    };

    let surface = unsafe {
        egl_lib.create_window_surface(egl_display, config, egl_window.as_ptr(), None)?
    };
    egl_lib.make_current(egl_display, Some(surface), Some(surface), Some(context))?;

    let gl = unsafe {
        glow::Context::from_loader_function(|name| {
            egl_lib
                .get_proc_address(name)
                .map(|p| p as *const _)
                .unwrap_or(ptr::null())
        })
    };

    use glow::HasContext;
    unsafe {
        println!("hello-egl-probe: GL_VENDOR     = {}", gl.get_parameter_string(glow::VENDOR));
        println!("hello-egl-probe: GL_RENDERER   = {}", gl.get_parameter_string(glow::RENDERER));
        println!("hello-egl-probe: GL_VERSION    = {}", gl.get_parameter_string(glow::VERSION));
        println!(
            "hello-egl-probe: GL_GLSL       = {}",
            gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION)
        );
    }

    let verdict = if gl_major >= 3 {
        "go/no-go: GO — GLES 3.x is available, Phase 3 can use egui_glow"
    } else {
        "go/no-go: NO-GO for GLES 3; Phase 3 needs an older egui or a different GL stack"
    };
    println!("hello-egl-probe: {verdict}");

    Ok(())
}

fn try_context(
    egl_lib: &egl::DynamicInstance<egl::EGL1_4>,
    egl_display: egl::Display,
    config: egl::Config,
    client_version: i32,
) -> Result<egl::Context, Box<dyn std::error::Error>> {
    let ctx_attrs = [egl::CONTEXT_CLIENT_VERSION, client_version, egl::NONE];
    let ctx = egl_lib.create_context(egl_display, config, None, &ctx_attrs)?;
    Ok(ctx)
}
