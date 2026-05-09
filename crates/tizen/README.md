# tizen

[![crates.io](https://img.shields.io/crates/v/tizen.svg)](https://crates.io/crates/tizen)
[![docs.rs](https://docs.rs/tizen/badge.svg)](https://docs.rs/tizen)

Idiomatic Rust bindings to Tizen platform APIs — umbrella crate.

This crate is a thin re-exporter. Each Tizen subsystem lives in its own focused
crate (`tizen-dlog`, `tizen-app`, …) and this crate aggregates them behind
cargo features so you opt into exactly what you need.

## Quick start

```toml
[dependencies]
tizen = { version = "0.1", features = ["dlog"] }
log = "0.4"
```

```rust
fn main() {
    tizen::dlog::init("MyApp").unwrap();
    log::info!("hello from rust");
    log::error!(target: "Network", "boom: {}", 42);
}
```

## Features

| Feature        | Module         | Status     | Backing crate                                        |
|----------------|----------------|------------|------------------------------------------------------|
| `dlog`         | `tizen::dlog`  | 🟢 alpha   | [`tizen-dlog`](https://crates.io/crates/tizen-dlog)  |
| `app`          | `tizen::app`   | 📝 planned | `tizen-app`                                          |
| `system-info`  | `tizen::system_info` | 📝 planned | `tizen-system-info`                          |
| `sensor`       | `tizen::sensor`| 📝 planned | `tizen-sensor`                                       |
| `bundle`       | `tizen::bundle`| 📝 planned | `tizen-bundle`                                       |
| `notification` | `tizen::notification` | 📝 planned | `tizen-notification`                          |

No features are on by default — opt in to the bindings you actually need.

## Umbrella vs direct dependency

Both work, pick whichever fits:

```toml
# Umbrella — single dep, choose features
tizen = { version = "0.1", features = ["dlog", "app"] }
```

```toml
# Direct — depend only on the bindings you use, skip the umbrella
tizen-dlog = "0.1"
tizen-app  = "0.1"
```

The umbrella is for ergonomic discovery and unified version pinning. The direct
crates are for users who want their `Cargo.toml` to spell out the exact
subsystems they touch.

## License

Dual-licensed under Apache-2.0 OR MIT, at your option. See the workspace root
for license texts.

Part of the [rust-tizen](https://github.com/smohantty/rust-tizen) project.
