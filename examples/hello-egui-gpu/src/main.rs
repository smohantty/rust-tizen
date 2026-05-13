//! egui hello-world on Tizen, GPU path.
//!
//! Composes `tizen-window` (Wayland surface), `tizen-egl` (wl_egl_window),
//! `khronos-egl` (libEGL load + context), `glow` (GLES 3 function pointers),
//! and `egui_glow` (the egui GL backend). The render loop runs an
//! animated egui UI on every `RedrawRequested`.
//!
//! ## Cross-compile + run
//!
//! ```sh
//! cd examples/hello-egui-gpu
//! cargo tizen build --release
//! # push the binary to the device, then on-device:
//! XDG_RUNTIME_DIR=/run WAYLAND_DISPLAY=wayland-0 /tmp/hello-egui-gpu
//! ```

use std::ptr;
use std::sync::Arc;
use std::time::Instant;

use glow::HasContext;
use khronos_egl as egl;
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use tizen::egl::EglWindow;
use tizen::window::{Display, Event, WindowBuilder};

fn main() -> std::process::ExitCode {
    if let Err(e) = run() {
        eprintln!("hello-egui-gpu: {e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open window.
    let mut display = Display::connect()?;
    let mut window = WindowBuilder::new()
        .title("hello-egui-gpu")
        .app_id("rust.tizen.hello-egui-gpu")
        .size(1920, 1080)
        .build(&display)?;
    display.roundtrip(&mut window)?;
    let (init_w, init_h) = window.size();
    println!("hello-egui-gpu: configured at {init_w}x{init_h}");

    // 2. Build wl_egl_window. SAFETY: `window` lives for the rest of
    //    `run`, and `egl_window` drops first (LIFO of bindings).
    let egl_window = unsafe { EglWindow::new(&window, init_w, init_h)? };

    // 3. Load libEGL.
    let egl_lib = unsafe { egl::DynamicInstance::<egl::EGL1_4>::load_required()? };

    // 4. EGL display + config.
    let wl_display_ptr = match display
        .display_handle()
        .map_err(|e| format!("display_handle: {e}"))?
        .as_raw()
    {
        RawDisplayHandle::Wayland(wl) => wl.display.as_ptr(),
        _ => return Err("display handle is not Wayland".into()),
    };
    let egl_display = unsafe {
        egl_lib
            .get_display(wl_display_ptr)
            .ok_or("eglGetDisplay returned NO_DISPLAY")?
    };
    let (major, minor) = egl_lib.initialize(egl_display)?;
    println!("hello-egui-gpu: EGL {major}.{minor}");
    egl_lib.bind_api(egl::OPENGL_ES_API)?;

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

    // GLES 3 first, fall back to 2.
    let (context, gl_major) = match try_context(&egl_lib, egl_display, config, 3) {
        Ok(c) => (c, 3),
        Err(_) => (try_context(&egl_lib, egl_display, config, 2)?, 2),
    };
    println!("hello-egui-gpu: GLES {gl_major} context");

    let egl_surface = unsafe {
        egl_lib.create_window_surface(egl_display, config, egl_window.as_ptr(), None)?
    };
    egl_lib.make_current(egl_display, Some(egl_surface), Some(egl_surface), Some(context))?;

    // 5. glow + egui.
    let gl = Arc::new(unsafe {
        glow::Context::from_loader_function(|name| {
            egl_lib
                .get_proc_address(name)
                .map(|p| p as *const _)
                .unwrap_or(ptr::null())
        })
    });
    unsafe {
        println!(
            "hello-egui-gpu: GL_VERSION = {}",
            gl.get_parameter_string(glow::VERSION)
        );
    }
    let mut painter = egui_glow::Painter::new(gl.clone(), "", None, false)
        .map_err(|e| format!("egui_glow::Painter::new: {e}"))?;
    let egui_ctx = egui::Context::default();

    // 6. Render loop state.
    let start = Instant::now();
    let mut frame: u64 = 0;
    let mut last_log = Instant::now();
    let mut frames_since_log: u64 = 0;
    window.request_redraw();

    display.run(&mut window, |window, event| match event {
        Event::Resized { width, height } => {
            let _ = egl_window.resize(width, height, 0, 0);
        }
        Event::RedrawRequested => {
            let (w, h) = window.size();
            let t = start.elapsed().as_secs_f32();

            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(w as f32, h as f32),
                )),
                time: Some(start.elapsed().as_secs_f64()),
                ..Default::default()
            };

            let full = egui_ctx.run(raw_input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("hello, Tizen!");
                    ui.label(format!("frame {frame}"));
                    ui.label(format!("elapsed {t:.2}s"));
                    ui.separator();
                    ui.label("rendered via tizen-window + tizen-egl + egui_glow.");
                });
                egui::Window::new("animation").show(ctx, |ui| {
                    let phase = t * 1.5;
                    let progress = (phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                    ui.add(egui::ProgressBar::new(progress).show_percentage());
                    ui.add_space(8.0);
                    ui.label(format!("sin(t·1.5) = {:+.3}", phase.sin()));
                });
            });

            let primitives = egui_ctx.tessellate(full.shapes, full.pixels_per_point);

            // Animated clear colour so we can tell the GL path is live
            // even without any input.
            let r = 0.05 + 0.05 * (t * 0.7).sin().abs();
            let g = 0.05 + 0.05 * (t * 1.1).sin().abs();
            let b = 0.10 + 0.05 * (t * 1.3).sin().abs();
            unsafe {
                gl.viewport(0, 0, w as i32, h as i32);
                gl.clear_color(r, g, b, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }
            painter.paint_and_update_textures(
                [w, h],
                full.pixels_per_point,
                &primitives,
                &full.textures_delta,
            );

            let _ = egl_lib.swap_buffers(egl_display, egl_surface);

            frame += 1;
            frames_since_log += 1;
            if last_log.elapsed().as_secs_f32() >= 1.0 {
                let fps = frames_since_log as f32 / last_log.elapsed().as_secs_f32();
                println!("hello-egui-gpu: frame={frame} fps≈{fps:.1}");
                frames_since_log = 0;
                last_log = Instant::now();
            }
            window.request_redraw();
        }
        _ => {}
    })?;

    Ok(())
}

fn try_context(
    egl_lib: &egl::DynamicInstance<egl::EGL1_4>,
    egl_display: egl::Display,
    config: egl::Config,
    client_version: i32,
) -> Result<egl::Context, Box<dyn std::error::Error>> {
    let attrs = [egl::CONTEXT_CLIENT_VERSION, client_version, egl::NONE];
    Ok(egl_lib.create_context(egl_display, config, None, &attrs)?)
}
