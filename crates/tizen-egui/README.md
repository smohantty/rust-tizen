# tizen-egui

egui integration for `rust-tizen` windows using EGL/GLES through
`egui_glow`.

This crate is the UI-framework-specific layer. `tizen-window` and
`tizen-egl` stay low-level and framework-agnostic; this crate owns the
egui input conversion, EGL context setup, glow context, painter,
resize handling, repaint scheduling, and buffer swaps.

For normal applications, use `run_native`:

```rust,no_run
use tizen_egui::{egui, NativeOptions};

tizen_egui::run_native(NativeOptions::default(), |ctx, _frame| {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("hello, Tizen!");
    });
})?;
# Ok::<(), tizen_egui::Error>(())
```

For advanced applications that own the Tizen event loop themselves, use
`TizenEguiGlow` directly.
