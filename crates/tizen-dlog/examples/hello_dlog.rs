//! In-crate smoke test for `tizen-dlog`. For a standalone consumer
//! template, see `examples/hello-dlog/` in the workspace root.

fn main() {
    tizen_dlog::init("HelloRust").expect("install logger");

    log::info!("hello from rust");
    log::warn!("warning: {} retries left", 3);
    log::error!(target: "Network", "boom: {}", 42);
}
