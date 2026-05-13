//! egui hello-world on Tizen, GPU path.
//!
//! Composes `tizen-window` (Wayland surface), `tizen-egl` (wl_egl_window),
//! `khronos-egl` (libEGL load + context), `glow` (GLES 3 function pointers),
//! and `egui_glow` (the egui GL backend). The render loop runs an
//! animated egui UI on every `RedrawRequested`.
//!
//! Boot-time milestones are printed with `[launch +NNN ms] label` so the
//! first-frame latency breakdown is visible in the log without any
//! external profiler. The summary line `first_frame_ms = N` is what to
//! grep for.
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
    // `LAUNCH` is captured as early as possible so the first milestone
    // is "main entered" rather than "after argv parsing." `Instant`
    // uses CLOCK_MONOTONIC under the hood.
    let launch = Instant::now();
    mark(launch, "main entered");
    if let Err(e) = run(launch) {
        eprintln!("hello-egui-gpu: {e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn mark(launch: Instant, label: &str) {
    let ms = launch.elapsed().as_secs_f64() * 1000.0;
    println!("[launch +{ms:>7.2} ms] {label}");
}

fn run(launch: Instant) -> Result<(), Box<dyn std::error::Error>> {
    let mut display = Display::connect()?;
    mark(launch, "Display::connect");

    let mut window = WindowBuilder::new()
        .title("hello-egui-gpu")
        .app_id("rust.tizen.hello-egui-gpu")
        .size(1920, 1080)
        .build(&display)?;
    mark(launch, "WindowBuilder::build");

    display.roundtrip(&mut window)?;
    let (init_w, init_h) = window.size();
    mark(launch, &format!("first configure ({init_w}x{init_h})"));

    // SAFETY: `window` lives for the rest of `run`, `egl_window` drops first.
    let egl_window = unsafe { EglWindow::new(&window, init_w, init_h)? };
    mark(launch, "EglWindow::new");

    let egl_lib = unsafe { egl::DynamicInstance::<egl::EGL1_4>::load_required()? };
    mark(launch, "libEGL loaded");

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
    let (egl_major, egl_minor) = egl_lib.initialize(egl_display)?;
    mark(launch, &format!("eglInitialize ({egl_major}.{egl_minor})"));
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

    let (context, gl_major) = match try_context(&egl_lib, egl_display, config, 3) {
        Ok(c) => (c, 3),
        Err(_) => (try_context(&egl_lib, egl_display, config, 2)?, 2),
    };
    mark(launch, &format!("eglCreateContext (GLES {gl_major})"));

    let egl_surface = unsafe {
        egl_lib.create_window_surface(egl_display, config, egl_window.as_ptr(), None)?
    };
    egl_lib.make_current(egl_display, Some(egl_surface), Some(egl_surface), Some(context))?;
    mark(launch, "eglMakeCurrent");

    let gl = Arc::new(unsafe {
        glow::Context::from_loader_function(|name| {
            egl_lib
                .get_proc_address(name)
                .map(|p| p as *const _)
                .unwrap_or(ptr::null())
        })
    });
    mark(launch, "glow::Context");

    let mut painter = egui_glow::Painter::new(gl.clone(), "", None, false)
        .map_err(|e| format!("egui_glow::Painter::new: {e}"))?;
    mark(launch, "egui_glow::Painter");

    let egui_ctx = egui::Context::default();

    let start = Instant::now();
    let mut frame: u64 = 0;
    let mut last_log = Instant::now();
    let mut frames_since_log: u64 = 0;
    let mut first_frame_done = false;
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

            if !first_frame_done {
                mark(launch, "first egui pass (tessellate ahead)");
            }
            let primitives = egui_ctx.tessellate(full.shapes, full.pixels_per_point);

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

            if !first_frame_done {
                let ms = launch.elapsed().as_secs_f64() * 1000.0;
                mark(launch, "first eglSwapBuffers returned");
                println!("hello-egui-gpu: first_frame_ms = {ms:.2}");
                first_frame_done = true;
            }

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
