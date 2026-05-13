# tizen-egui

egui integration for `rust-tizen` windows using EGL/GLES through
`egui_glow`.

This crate is the UI-framework-specific layer. `tizen-window` and
`tizen-egl` stay low-level and framework-agnostic; this crate owns the
egui input conversion, EGL context setup, glow context, painter,
resize handling, repaint scheduling, and buffer swaps.

For normal applications, implement `App` and pass it to `run_native`.
The app model is inspired by `eframe`, but this crate does not depend
on or implement upstream `eframe`.

```rust,no_run
use tizen_egui::{egui, App, Frame, NativeOptions};

struct MyApp;

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("hello, Tizen!");
        });
    }
}

tizen_egui::run_native(
    "hello",
    NativeOptions {
        title: "hello".into(),
        app_id: "rust.tizen.hello".into(),
        size: (1920, 1080),
        ..Default::default()
    },
    Box::new(|_cc| Ok(Box::new(MyApp))),
)?;
# Ok::<(), tizen_egui::Error>(())
```

For advanced applications that own the Tizen event loop themselves, use
`TizenEguiGlow` directly.
