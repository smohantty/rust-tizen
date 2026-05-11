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
tizen = { git = "https://github.com/smohantty/rust-tizen.git", features = ["dlog"] }
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

| Feature      | Module        | Backing crate                                          |
|--------------|---------------|--------------------------------------------------------|
| `dlog`       | `tizen::dlog` | [`tizen-dlog`](https://crates.io/crates/tizen-dlog)    |
| `app`        | `tizen::app`  | [`tizen-app`](https://crates.io/crates/tizen-app)      |
| `app-tokio`  | `tizen::app`  | [`tizen-app`](https://crates.io/crates/tizen-app) + tokio |

No features are on by default.

Equivalent direct-dep form, if you'd rather skip the umbrella:

```toml
tizen-dlog = { git = "https://github.com/smohantty/rust-tizen.git" }
tizen-app  = { git = "https://github.com/smohantty/rust-tizen.git" }
```

## License

Dual-licensed under Apache-2.0 OR MIT, at your option. See the workspace root
for license texts.

Part of the [rust-tizen](https://github.com/smohantty/rust-tizen) project.
