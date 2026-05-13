//! Minimal end-to-end test for `tizen-input`.
//!
//! Cross-compile and deploy to a Tizen device:
//!
//! ```sh
//! cd examples/hello-input
//! cargo tizen build -A armv7l --release       # or -A aarch64
//! cargo tizen rpm   -A armv7l --release       # for an RPM
//! ```
//!
//! Run on the device (one of):
//!
//! ```sh
//! WAYLAND_DISPLAY=wayland-0 ./hello-input XF86Back
//! WAYLAND_DISPLAY=wayland-0 ./hello-input XF86Back XF86Home Return
//! ```
//!
//! Each argument is injected as a *press → release* keystroke with a short
//! gap between keys.
//!
//! Watch the compositor pick the event up via `dlogutil enlightenment:V` on
//! the device, or have an interactive app focused (the launcher reacts to
//! `XF86Back` / `XF86Home` on most Tizen TV / mobile profiles).

use std::{env, thread, time::Duration};

use tizen::input::{DeviceType, Error, InputGenerator, KeyState};

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().skip(1).collect();
    let keys: Vec<&str> = if args.is_empty() {
        vec!["XF86Back"]
    } else {
        args.iter().map(String::as_str).collect()
    };

    let mut gen = InputGenerator::builder()
        .name("hello-input")
        .device(DeviceType::KEYBOARD)
        .open()?;

    println!("hello-input: injecting {} key(s)", keys.len());
    for name in keys {
        println!("  -> {name}");
        gen.key(name, KeyState::Pressed)?;
        thread::sleep(Duration::from_millis(30));
        gen.key(name, KeyState::Released)?;
        thread::sleep(Duration::from_millis(120));
    }

    println!("hello-input: done");
    Ok(())
}
