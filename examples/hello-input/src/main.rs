//! Drive a Tizen compositor's focused launcher / dialog with synthetic
//! input via `tizen-input`.
//!
//! ## Usage
//!
//! ```text
//! hello-input <subcommand> [args...]
//!
//! Subcommands:
//!   info                 Probe compositor: max_touch_count, accepted classes
//!   help                 Print this help
//!
//!   nav <preset|KEY [KEY ...]>
//!                        Keyboard injection. `preset` is one of:
//!                        luud, rrrd, box, back, home.
//!                        Otherwise tokens are X11 keysym names sent in order.
//!
//!   key <NAME> [press|release]
//!                        One key. With no phase, sends press+release.
//!
//!   click [X Y]          Primary mouse click at (X, Y). Default: 960 540
//!                        (centre of 1920×1080). Visible on Tizen TV.
//!
//!   move <X> <Y>         Move the pointer to (X, Y), no click.
//!
//!   tap [X Y] [--finger N]
//!                        Touch tap at (X, Y) on finger slot N (default 0).
//!                        Dispatches through the compositor; visible only if
//!                        a touch-listening surface is in focus.
//! ```
//!
//! ## Cross-compile and deploy
//!
//! ```sh
//! cd examples/hello-input
//! cargo tizen build -A armv7l --release        # or -A aarch64
//! ```

use std::{env, thread, time::Duration};

use tizen::input::{
    DeviceType, Error, InputGenerator, KeyState, PointerButton, TouchPhase,
};

/// Named keyboard navigation presets.
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

const DEFAULT_X: u32 = 960;
const DEFAULT_Y: u32 = 540;

fn main() -> std::process::ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    let (cmd, rest) = match argv.split_first() {
        Some((c, r)) => (c.as_str(), r),
        None => {
            print_help();
            return std::process::ExitCode::SUCCESS;
        }
    };

    let result = match cmd {
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        "info" | "probe" => info_cmd(),
        "nav" => nav_cmd(rest),
        "key" => key_cmd(rest),
        "click" => click_cmd(rest),
        "move" => move_cmd(rest),
        "tap" => tap_cmd(rest),
        other => {
            eprintln!("hello-input: unknown subcommand `{other}`\n");
            print_help();
            return std::process::ExitCode::from(2);
        }
    };

    if let Err(e) = result {
        eprintln!("hello-input: error: {e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

// --- subcommands ----------------------------------------------------------

fn info_cmd() -> Result<(), Error> {
    let gen = open_all_classes("hello-input-probe")?;
    println!("hello-input: probe");
    println!("  init_generator(POINTER|KEYBOARD|TOUCHSCREEN): accepted");
    match gen.max_touch_count() {
        Some(n) => println!("  max_touch_count: {n}"),
        None => println!("  max_touch_count: (not reported)"),
    }
    Ok(())
}

fn nav_cmd(args: &[String]) -> Result<(), Error> {
    let keys: Vec<String> = match args.first().map(String::as_str) {
        None => {
            eprintln!("hello-input: `nav` needs a preset or one or more key names");
            return Ok(());
        }
        Some(name) if args.len() == 1 && find_preset(name).is_some() => preset_keys(name),
        _ => args.to_vec(),
    };

    let mut gen = open_all_classes("hello-input")?;
    println!("hello-input: nav — injecting {} key(s):", keys.len());
    for k in &keys {
        println!("  -> {k}");
        gen.key(k, KeyState::Pressed)?;
        thread::sleep(Duration::from_millis(40));
        gen.key(k, KeyState::Released)?;
        thread::sleep(Duration::from_millis(250));
    }
    println!("hello-input: done");
    Ok(())
}

fn key_cmd(args: &[String]) -> Result<(), Error> {
    let Some(name) = args.first() else {
        eprintln!("hello-input: `key` needs a key name (e.g. XF86Back)");
        return Ok(());
    };
    let phase = args.get(1).map(String::as_str);

    let mut gen = open_all_classes("hello-input")?;
    match phase {
        None => {
            println!("hello-input: key {name} (press + release)");
            gen.key(name, KeyState::Pressed)?;
            thread::sleep(Duration::from_millis(40));
            gen.key(name, KeyState::Released)?;
        }
        Some("press") => {
            println!("hello-input: key {name} press");
            gen.key(name, KeyState::Pressed)?;
        }
        Some("release") => {
            println!("hello-input: key {name} release");
            gen.key(name, KeyState::Released)?;
        }
        Some(other) => {
            eprintln!("hello-input: `key` phase must be `press` or `release` (got `{other}`)");
        }
    }
    Ok(())
}

fn click_cmd(args: &[String]) -> Result<(), Error> {
    let x = parse_coord(args.first(), DEFAULT_X);
    let y = parse_coord(args.get(1), DEFAULT_Y);
    let mut gen = open_all_classes("hello-input")?;
    println!("hello-input: click at ({x}, {y})");
    gen.click(PointerButton::Left, x, y)?;
    println!("hello-input: done");
    Ok(())
}

fn move_cmd(args: &[String]) -> Result<(), Error> {
    let (Some(xs), Some(ys)) = (args.first(), args.get(1)) else {
        eprintln!("hello-input: `move` needs X and Y");
        return Ok(());
    };
    let x = parse_coord(Some(xs), DEFAULT_X);
    let y = parse_coord(Some(ys), DEFAULT_Y);
    let mut gen = open_all_classes("hello-input")?;
    println!("hello-input: move pointer to ({x}, {y})");
    gen.move_pointer(x, y)?;
    println!("hello-input: done");
    Ok(())
}

fn tap_cmd(args: &[String]) -> Result<(), Error> {
    // Parse optional --finger N before positional coords.
    let mut finger = 0u32;
    let mut positional: Vec<&str> = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--finger" => {
                if let Some(n) = iter.next() {
                    finger = n.parse().unwrap_or(0);
                }
            }
            other => positional.push(other),
        }
    }
    let x = positional
        .first()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(DEFAULT_X);
    let y = positional
        .get(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(DEFAULT_Y);

    let mut gen = open_all_classes("hello-input")?;
    println!("hello-input: tap at ({x}, {y}) on finger slot {finger}");
    gen.touch(finger, TouchPhase::Begin, x, y)?;
    thread::sleep(Duration::from_millis(50));
    gen.touch(finger, TouchPhase::End, x, y)?;
    println!("hello-input: done");
    Ok(())
}

// --- helpers --------------------------------------------------------------

fn open_all_classes(name: &str) -> Result<InputGenerator, Error> {
    InputGenerator::builder()
        .name(name)
        .device(DeviceType::KEYBOARD | DeviceType::POINTER | DeviceType::TOUCHSCREEN)
        .open()
}

fn parse_coord(arg: Option<&String>, fallback: u32) -> u32 {
    arg.and_then(|s| s.parse::<u32>().ok()).unwrap_or(fallback)
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

fn print_help() {
    print!(
        r#"hello-input — synthetic input injection for Tizen Wayland

USAGE:
    hello-input <subcommand> [args...]

SUBCOMMANDS:
    info                          Probe compositor capabilities + max_touch_count
    help                          Print this help

    nav <preset|KEY ...>          Keyboard navigation.
                                  Presets: luud, rrrd, box, back, home.
                                  Otherwise tokens are X11 keysym names in order.

    key <NAME> [press|release]    One key (defaults to press+release pair).

    click [X Y]                   Primary mouse click at (X, Y). Default 960 540.
    move  <X> <Y>                 Move pointer to (X, Y), no click.
    tap   [X Y] [--finger N]      Touch tap at (X, Y) on finger slot N (default 0).

EXAMPLES:
    hello-input info
    hello-input nav luud
    hello-input nav Up Up Right Return
    hello-input key XF86Back
    hello-input click 200 400
    hello-input tap 960 540 --finger 1
"#
    );
}
