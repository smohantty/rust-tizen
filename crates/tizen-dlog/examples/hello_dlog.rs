//! Run on a Tizen device via cargo-tizen:
//!
//!     cargo tizen build -A armv7l --release --example hello_dlog
//!     cargo tizen install -A armv7l --release          # for a TPK
//!     # or, manually:
//!     sdb push target/armv7-unknown-linux-gnueabi/release/examples/hello_dlog /tmp/
//!     sdb shell /tmp/hello_dlog
//!
//! Then in another shell on the device:
//!
//!     dlogutil HelloRust:* Network:* '*:S'
//!
//! You should see one INFO line tagged `HelloRust`, one WARN line tagged `HelloRust`,
//! and one ERROR line tagged `Network`.
//!
//! On non-Tizen hosts this example compiles to a stub so `cargo build --examples`
//! works in dev environments where `libdlog.so` is not available. The activation
//! cfg matches what `tizen-dlog-sys` uses for linking — `--cfg tizen` (set by
//! cargo-tizen) or `feature = "tizen"`.

#[cfg(any(tizen, feature = "tizen"))]
fn main() {
    tizen_dlog::init("HelloRust").expect("install logger");

    log::info!("hello from rust");
    log::warn!("warning: {} retries left", 3);
    log::error!(target: "Network", "boom: {}", 42);
}

#[cfg(not(any(tizen, feature = "tizen")))]
fn main() {
    eprintln!(
        "hello_dlog: this example is a no-op off-device. \
         Build with cargo-tizen, or pass --cfg tizen via RUSTFLAGS, \
         or enable the `tizen` cargo feature."
    );
}
