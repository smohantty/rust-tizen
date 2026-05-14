# tizen-eframe

eframe-style runner for egui applications on Tizen, backed by EGL/GLES
through `egui_glow`. The public API (`App`, `Frame`, `NativeOptions`,
`CreationContext`, `AppCreator`, `run_native`) mirrors upstream
`eframe`'s shape with a trimmed feature set, so eframe boilerplate
compiles unchanged.

## Use in an app

Add the dep with a package rename so the source reads as plain eframe
code:

```toml
[dependencies]
eframe = { package = "tizen-eframe", git = "https://github.com/smohantty/rust-tizen" }
```

```rust,no_run
use eframe::{egui, App, Frame, NativeOptions};

struct MyApp;

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("hello, Tizen!");
        });
    }
}

eframe::run_native(
    "hello",
    NativeOptions {
        title: "hello".into(),
        app_id: "rust.tizen.hello".into(),
        size: (1920, 1080),
        ..Default::default()
    },
    Box::new(|_cc| Ok(Box::new(MyApp))),
)?;
# Ok::<(), eframe::Error>(())
```

## What's owned

This crate owns the egui input conversion, EGL context setup, glow
context, `egui_glow::Painter`, resize handling, repaint scheduling, and
buffer swaps. `tizen-window` and `tizen-egl` stay low-level and
framework-agnostic.

## Advanced

For applications that own the Tizen event loop themselves, use
`TizenEguiGlow` directly.
