//! egui hello-world on Tizen, GPU path.
//!
//! This example uses the app-facing `tizen-egui` runner. The runner owns
//! the Tizen window, EGL/GLES setup, `egui_glow::Painter`, input
//! conversion, repaint loop, and buffer swaps.
//!
//! ## Cross-compile + run
//!
//! ```sh
//! cd examples/hello-egui-gpu
//! cargo tizen build --release
//! # push the binary to the device, then on-device:
//! XDG_RUNTIME_DIR=/run WAYLAND_DISPLAY=wayland-0 /tmp/hello-egui-gpu
//! ```

use tizen_egui::{egui, NativeOptions};

fn main() -> std::process::ExitCode {
    let options = NativeOptions {
        title: "hello-egui-gpu".to_owned(),
        app_id: "rust.tizen.hello-egui-gpu".to_owned(),
        size: (1920, 1080),
        continuous_repaint: true,
        ..Default::default()
    };

    match tizen_egui::run_native(options, |ctx, frame| {
        let elapsed = frame.elapsed().as_secs_f32();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("hello, Tizen!");
            ui.label(format!("frame {}", frame.frame_nr()));
            ui.label(format!("elapsed {elapsed:.2}s"));
            ui.separator();
            ui.label("rendered via tizen-egui + egui_glow.");
        });

        egui::Window::new("animation").show(ctx, |ui| {
            let phase = elapsed * 1.5;
            let progress = (phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            ui.add(egui::ProgressBar::new(progress).show_percentage());
            ui.add_space(8.0);
            ui.label(format!("sin(t * 1.5) = {:+.3}", phase.sin()));
        });
    }) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hello-egui-gpu: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
