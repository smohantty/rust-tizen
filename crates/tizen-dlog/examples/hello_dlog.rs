//! Run on a Tizen device:
//!
//!     cargo build --release --target armv7l-tizen-linux-gnueabi --example hello_dlog
//!     sdb push target/armv7l-tizen-linux-gnueabi/release/examples/hello_dlog /tmp/
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
//! works in dev environments where `libdlog.so` is not available.

#[cfg(target_vendor = "tizen")]
fn main() {
    tizen_dlog::init("HelloRust").expect("install logger");

    log::info!("hello from rust");
    log::warn!("warning: {} retries left", 3);
    log::error!(target: "Network", "boom: {}", 42);
}

#[cfg(not(target_vendor = "tizen"))]
fn main() {
    eprintln!(
        "hello_dlog: this example is a no-op off-device. \
         Cross-compile for a Tizen target (e.g. armv7l-tizen-linux-gnueabi) and run on a device."
    );
}
