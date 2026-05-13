//! Capture a screenshot from a Tizen Wayland compositor and save it as a
//! PNG in `/tmp/` so you can pull it back with `rsdb` and view it.
//!
//! ## Usage
//!
//! ```text
//! hello-screenshot <subcommand> [args...]
//!
//! Subcommands:
//!   shoot [-o PATH]              Full-screen capture. Default path: /tmp/screenshot.png
//!   area  X Y W H [-o PATH]      Capture (W, H) region starting at (X, Y).
//!                                 Default path: /tmp/screenshot-area.png
//!   info                         Probe: compositor + max touch count
//!   help                         Print this help
//! ```
//!
//! ## Cross-compile and deploy
//!
//! ```sh
//! cd examples/hello-screenshot
//! cargo tizen build -A armv7l --release    # or -A aarch64
//! ```

use std::{env, fs::File, io::BufWriter, path::PathBuf, time::Instant};

use tizen::screenshot::{ScreenCapturer, ScreenFrame};

const DEFAULT_FULL: &str = "/tmp/screenshot.png";
const DEFAULT_AREA: &str = "/tmp/screenshot-area.png";

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
        "shoot" | "full" => shoot_cmd(rest),
        "area" => area_cmd(rest),
        other => {
            eprintln!("hello-screenshot: unknown subcommand `{other}`\n");
            print_help();
            return std::process::ExitCode::from(2);
        }
    };

    if let Err(e) = result {
        eprintln!("hello-screenshot: error: {e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn info_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let _cap = ScreenCapturer::new()?;
    println!("hello-screenshot: probe");
    println!("  connect + bind tizen_screenshooter: ok");
    Ok(())
}

fn shoot_cmd(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let out = parse_output(args, DEFAULT_FULL);
    let mut cap = ScreenCapturer::new()?;
    let t0 = Instant::now();
    let frame = cap.shoot()?;
    let took = t0.elapsed();
    println!(
        "hello-screenshot: captured {}x{} stride={} ({} bytes) in {:?}",
        frame.width(),
        frame.height(),
        frame.stride(),
        frame.bytes().len(),
        took
    );
    save_png(&frame, &out)?;
    Ok(())
}

fn area_cmd(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 4 {
        return Err("`area` needs X Y W H".into());
    }
    let x: i32 = args[0].parse()?;
    let y: i32 = args[1].parse()?;
    let w: i32 = args[2].parse()?;
    let h: i32 = args[3].parse()?;
    let out = parse_output(&args[4..], DEFAULT_AREA);

    let mut cap = ScreenCapturer::new()?;
    let t0 = Instant::now();
    let frame = cap.shoot_area(x, y, w, h)?;
    let took = t0.elapsed();
    println!(
        "hello-screenshot: area ({x},{y} {w}x{h}) → {}x{} stride={} in {:?}",
        frame.width(),
        frame.height(),
        frame.stride(),
        took
    );
    save_png(&frame, &out)?;
    Ok(())
}

fn parse_output(args: &[String], default: &str) -> PathBuf {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == "-o" || a == "--output" {
            if let Some(p) = iter.next() {
                return PathBuf::from(p);
            }
        }
    }
    PathBuf::from(default)
}

/// Encode the XRGB8888 frame as PNG. The compositor returns pixels in
/// BGRA byte order in memory (X=0xff R G B on most ARM endianess after
/// XRGB8888 little-endian load), but the simplest correct mapping is:
///
///   byte 0 = B, byte 1 = G, byte 2 = R, byte 3 = X (ignored)
///
/// So we reshuffle to RGB triples on the fly and tell `png` to write a
/// 24-bit RGB image. This keeps the output viewer-friendly without
/// chasing colour-space mishaps.
fn save_png(frame: &ScreenFrame, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let w = frame.width() as usize;
    let h = frame.height() as usize;
    let stride = frame.stride() as usize;
    let src = frame.bytes();

    let mut rgb = Vec::with_capacity(w * h * 3);
    for row in 0..h {
        let line = &src[row * stride..row * stride + w * 4];
        for px in line.chunks_exact(4) {
            // XRGB8888 little-endian in memory == BGRX byte order:
            //   px[0]=B  px[1]=G  px[2]=R  px[3]=X (alpha/pad)
            rgb.push(px[2]); // R
            rgb.push(px[1]); // G
            rgb.push(px[0]); // B
        }
    }

    let file = File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), frame.width(), frame.height());
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&rgb)?;
    println!("hello-screenshot: wrote {}", path.display());
    Ok(())
}

fn print_help() {
    print!(
        r#"hello-screenshot — capture a Tizen Wayland screen to PNG

USAGE:
    hello-screenshot <subcommand> [args...]

SUBCOMMANDS:
    shoot [-o PATH]               Full screen → PNG (default {DEFAULT_FULL})
    area X Y W H [-o PATH]        Region capture → PNG (default {DEFAULT_AREA})
    info                          Probe compositor + screenshooter binding
    help                          Print this help

EXAMPLES:
    hello-screenshot shoot
    hello-screenshot shoot -o /tmp/desktop.png
    hello-screenshot area 100 100 400 300
"#
    );
}
