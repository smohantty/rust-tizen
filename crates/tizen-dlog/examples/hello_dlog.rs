//! In-crate smoke test for `tizen-dlog`. For a standalone consumer
//! template, see `examples/hello-dlog/` in the workspace root.

#[cfg(tizen)]
fn main() {
    tizen_dlog::init("HelloRust").expect("install logger");

    log::info!("hello from rust");
    log::warn!("warning: {} retries left", 3);
    log::error!(target: "Network", "boom: {}", 42);
}

#[cfg(not(tizen))]
fn main() {
    eprintln!(
        "hello_dlog: this example is a no-op off-device. \
         Cross-compile with `cargo tizen build` and run on a device."
    );
}
