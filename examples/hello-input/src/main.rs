//! Drive a Tizen compositor's focused launcher / dialog with synthetic
//! D-pad navigation via `tizen-input`.
//!
//! Cross-compile and deploy:
//!
//! ```sh
//! cd examples/hello-input
//! cargo tizen build -A armv7l --release       # or -A aarch64
//! ```
//!
//! Run on the device with one of:
//!
//! ```sh
//! /tmp/hello-input                    # default preset (luud)
//! /tmp/hello-input luud               # Left Up Up Down Return
//! /tmp/hello-input rrrd               # Right Right Right Down Return
//! /tmp/hello-input box                # walk a clockwise box on the launcher
//! /tmp/hello-input back               # one XF86Back press
//! /tmp/hello-input random             # 4–8 random arrow keys + Return
//! /tmp/hello-input Up Up Right Return # explicit key sequence
//! ```
//!
//! Keys are X11 keysym names — the compositor maps them to keycodes via
//! its own xkb table. `Up`/`Down`/`Left`/`Right`/`Return` work on the TV
//! profile; `XF86Back` and `XF86Home` are the TV remote's back and home.

use std::{env, thread, time::Duration};

use tizen::input::{DeviceType, Error, InputGenerator, KeyState};

/// Named navigation presets. Each is a sequence of X11 keysym names.
const PRESETS: &[(&str, &[&str])] = &[
    ("luud", &["Left", "Up", "Up", "Down", "Return"]),
    ("rrrd", &["Right", "Right", "Right", "Down", "Return"]),
    (
        "box",
        &[
            "Right", "Right", "Down", "Down", "Left", "Left", "Up", "Up",
        ],
    ),
    ("back", &["XF86Back"]),
    ("home", &["XF86Home"]),
];

const DIRS: &[&str] = &["Up", "Down", "Left", "Right"];

fn main() -> Result<(), Error> {
    let argv: Vec<String> = env::args().skip(1).collect();
    let keys: Vec<String> = resolve_keys(&argv);

    let mut gen = InputGenerator::builder()
        .name("hello-input")
        .device(DeviceType::KEYBOARD)
        .open()?;

    println!("hello-input: injecting {} key(s):", keys.len());
    for k in &keys {
        println!("  -> {k}");
        gen.key(k, KeyState::Pressed)?;
        thread::sleep(Duration::from_millis(40));
        gen.key(k, KeyState::Released)?;
        // Visible gap between keys so the on-screen focus animation can
        // actually catch up with the synthetic navigation.
        thread::sleep(Duration::from_millis(250));
    }
    println!("hello-input: done");
    Ok(())
}

/// Decide what to inject based on argv:
///
/// * empty → the `luud` preset
/// * single token matching a preset name → that preset
/// * `random` → 4–8 random arrows + Return
/// * anything else → treat each argv token as a key name verbatim
fn resolve_keys(argv: &[String]) -> Vec<String> {
    match argv.first().map(String::as_str) {
        None => preset_keys("luud"),
        Some("random") => random_sequence(),
        Some(name) if argv.len() == 1 && find_preset(name).is_some() => preset_keys(name),
        _ => argv.to_vec(),
    }
}

fn find_preset(name: &str) -> Option<&'static [&'static str]> {
    PRESETS
        .iter()
        .find_map(|(n, v)| (*n == name).then_some(*v))
}

fn preset_keys(name: &str) -> Vec<String> {
    find_preset(name)
        .unwrap_or(&[])
        .iter()
        .map(|&s| s.to_string())
        .collect()
}

/// Tiny xorshift seeded by `SystemTime` — keeps the example dep-free
/// (no `rand` crate) while still giving a different walk per invocation.
fn random_sequence() -> Vec<String> {
    let mut s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
        .max(1);
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };

    let len = 4 + (next() % 5) as usize; // 4..=8 directional keys
    let mut keys: Vec<String> = (0..len)
        .map(|_| DIRS[(next() as usize) % DIRS.len()].to_string())
        .collect();
    keys.push("Return".into());
    keys
}
