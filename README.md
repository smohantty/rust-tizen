# rust-tizen

Idiomatic Rust bindings to the Tizen platform — built incrementally as a monorepo of focused, single-purpose crates.

> **Status: early.** `tizen-dlog` (logging) is the inaugural binding. More are planned. Contributions welcome — see [`CONTRIBUTING.md`](./CONTRIBUTING.md).

## Why

Tizen ships a rich set of native C APIs (logging, app lifecycle, sensors, system info, …) but no first-class Rust story. Today, integrating Rust into a Tizen app means hand-writing FFI for every library you touch. This project's goal is to make that integration *one cargo line per subsystem*:

```toml
[dependencies]
tizen-dlog = "0.1"
# tizen-app = "0.1"          # later
# tizen-system-info = "0.1"  # later
```

…with safe wrappers, `log`/`tracing` integration where applicable, and consistent conventions across every binding.

## Crates

| Crate              | Version   | Status     | Tizen library               | Purpose                              |
|--------------------|-----------|------------|-----------------------------|--------------------------------------|
| [`tizen-dlog`]     | `0.1.0`   | 🟢 alpha   | `libdlog.so`                | `log` facade → Tizen `dlogutil`      |
| [`tizen-dlog-sys`] | `0.1.0`   | 🟢 alpha   | `libdlog.so`                | Raw FFI bindings for `dlog.h`        |
| `tizen-app`        | —         | 📝 planned | `libcapi-appfw-application` | App lifecycle, intents, events       |
| `tizen-system-info`| —         | 📝 planned | `libcapi-system-info`       | Device capabilities, OS metadata     |
| `tizen-sensor`     | —         | 📝 planned | `libcapi-system-sensor`     | Accelerometer, gyro, light, …        |
| `tizen-bundle`     | —         | 📝 planned | `libbundle`                 | Bundle (key-value) IPC payloads      |
| `tizen-notification`| —        | 📝 planned | `libnotification`           | System notifications                 |

**Status legend:** 📝 planned · 🟡 in progress · 🟢 alpha · 🔵 beta · ✅ stable

[`tizen-dlog`]: ./crates/tizen-dlog
[`tizen-dlog-sys`]: ./crates/tizen-dlog-sys

## Quick start (with `tizen-dlog`)

```toml
[dependencies]
log = "0.4"
tizen-dlog = "0.1"
```

```rust
fn main() {
    tizen_dlog::init("MyApp").expect("install logger");
    log::info!("hello from rust");
    log::error!(target: "Network", "boom: {}", 42);
}
```

On a Tizen device, view the output with `dlogutil MyApp:* Network:* '*:S'`.

See the [`tizen-dlog` crate README](./crates/tizen-dlog/README.md) for the full API.

## Building for Tizen

You'll need a Tizen sysroot (Tizen Studio or a GBS rootstrap) and a `.cargo/config.toml`
that points the linker at it. Step-by-step instructions:
[`docs/tizen-target-setup.md`](./docs/tizen-target-setup.md).

Quick version:

```bash
rustup target add armv7l-tizen-linux-gnueabi   # or use a custom target JSON
cargo build --release --target armv7l-tizen-linux-gnueabi
sdb push target/armv7l-tizen-linux-gnueabi/release/<your-binary> /tmp/
sdb shell /tmp/<your-binary>
```

## Project layout

```
rust-tizen/
├── Cargo.toml                  # workspace manifest
├── crates/                     # all sub-crates
│   ├── tizen-dlog-sys/         # raw FFI for libdlog
│   └── tizen-dlog/             # safe wrapper + log::Log impl
├── docs/
│   ├── adding-a-binding.md     # how to contribute a new tizen-foo crate
│   └── tizen-target-setup.md   # cross-compile setup
├── CONTRIBUTING.md
└── README.md
```

Every binding follows the same shape:

- `tizen-foo-sys/` — hand-written `extern "C"` declarations, `#![no_std]`, no deps beyond `core`.
- `tizen-foo/` — safe wrapper, idiomatic Rust API, optional `log`/`tracing` integration.

## Versioning

All crates currently ship at the **same workspace version** (`0.1.0`). This keeps
the release process simple while the ecosystem is young. Once individual crates
mature at different rates, we'll move to per-crate versioning.

## MSRV

Rust **1.80** or newer. We may bump conservatively when needed; bumps will be
documented in `CHANGELOG.md` for each affected crate.

## Contributing

We're actively looking for contributors who can wrap additional Tizen APIs.
The recipe for adding a new binding is short and well-defined — see
[`docs/adding-a-binding.md`](./docs/adding-a-binding.md) and
[`CONTRIBUTING.md`](./CONTRIBUTING.md).

Concrete asks:

- **`tizen-app`** — wrap `libcapi-appfw-application` so Rust apps can hook into
  the Tizen app lifecycle (create, pause, resume, terminate, app_control events).
- **`tizen-system-info`** — wrap `libcapi-system-info` to expose device model, OS
  version, and runtime capabilities.
- **Cross-compile CI** — figure out how to run `cargo check` against a Tizen
  sysroot in GitHub Actions (currently only host CI runs).

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](./LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](./LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in this project, as defined in the
Apache-2.0 license, shall be dual-licensed as above, without any additional
terms or conditions.
