//! Minimal EFL-free Tizen Wayland window.
//!
//! Opens a 640x480 window via zxdg_shell_v6 + wtz_shell, fills it with
//! a solid colour (BGRX), and runs until the compositor signals close.
//!
//! ## Usage
//!
//! ```sh
//! hello-window                  # default blue, 640x480
//! hello-window red              # named colour
//! hello-window 0x008B0000       # raw BGRX value
//! ```
//!
//! ## Cross-compile and deploy
//!
//! ```sh
//! cd examples/hello-window
//! cargo tizen build -A armv7l --release    # or -A aarch64
//! ```

use std::env;

use tizen::window::{Display, Event, WindowBuilder};

fn main() -> std::process::ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    let colour = parse_colour(argv.first().map(String::as_str));

    let mut display = match Display::connect() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("hello-window: connect: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut window = match WindowBuilder::new()
        .title("hello-window")
        .app_id("rust.tizen.hello-window")
        .size(640, 480)
        .build(&display)
    {
        Ok(w) => w,
        Err(e) => {
            eprintln!("hello-window: build: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    // Wait for the first configure so we know our real size.
    if let Err(e) = display.roundtrip(&mut window) {
        eprintln!("hello-window: initial roundtrip: {e}");
        return std::process::ExitCode::FAILURE;
    }

    let (w, h) = window.size();
    println!(
        "hello-window: configured at {w}x{h}, painting BGRX=0x{colour:08X}"
    );
    window.fill_solid(colour);

    let run_result = display.run(&mut window, |window, event| match event {
        Event::Resized { width, height } => {
            println!("hello-window: resized to {width}x{height}");
            window.fill_solid(colour);
        }
        Event::RedrawRequested => window.fill_solid(colour),
        Event::CursorEntered { x, y } => println!("hello-window: cursor enter at {x:.1},{y:.1}"),
        Event::CursorLeft => println!("hello-window: cursor leave"),
        Event::CursorMoved { x, y } => println!("hello-window: cursor at {x:.1},{y:.1}"),
        Event::MouseInput { button, pressed } => {
            let what = if pressed { "press" } else { "release" };
            println!("hello-window: mouse {what} {button:?}");
        }
        Event::MouseWheel { dx, dy } => println!("hello-window: wheel dx={dx:.2} dy={dy:.2}"),
        Event::Focused(focused) => println!("hello-window: focus={focused}"),
        Event::KeyboardInput {
            keycode, pressed, ..
        } => {
            let what = if pressed { "press" } else { "release" };
            println!("hello-window: key {what} keycode={keycode}");
        }
        _ => {}
    });

    if let Err(e) = run_result {
        eprintln!("hello-window: dispatch: {e}");
        return std::process::ExitCode::FAILURE;
    }

    println!("hello-window: close requested by compositor, exiting");
    std::process::ExitCode::SUCCESS
}

/// Resolve the colour argument: a named colour, a 0x-prefixed BGRX hex
/// value, a bare hex string, or the default blue.
fn parse_colour(arg: Option<&str>) -> u32 {
    let Some(s) = arg else {
        return 0x001E40AF; // BGRX deep blue (R=0xAF, G=0x40, B=0x1E, X=0)
    };

    // Named colours (BGRX layout — byte 0 = B, byte 1 = G, byte 2 = R).
    match s.to_ascii_lowercase().as_str() {
        "blue" => return 0x001E40AF,
        "red" => return 0x000000B0,
        "green" => return 0x0000B000,
        "white" => return 0x00FFFFFF,
        "black" => return 0x00000000,
        "magenta" => return 0x00B000B0,
        _ => {}
    }

    let hex = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    u32::from_str_radix(hex, 16).unwrap_or(0x001E40AF)
}
