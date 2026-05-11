# rust-tizen

Idiomatic Rust bindings to the Tizen platform — built incrementally as a monorepo of focused, single-purpose crates.

> **Status: early.** `tizen-dlog` (logging) is the inaugural binding. More are planned. Contributions welcome — see [`CONTRIBUTING.md`](./CONTRIBUTING.md).

## Why

Tizen ships a rich set of native C APIs (logging, app lifecycle, sensors, system info, …) but no first-class Rust story. Today, integrating Rust into a Tizen app means hand-writing FFI for every library you touch. This project's goal is to make that integration *one cargo line per subsystem*:

```toml
[dependencies]
# Umbrella: opt into the bindings you need by feature
tizen = { version = "0.1", features = ["dlog"] }
```

…with safe wrappers, `log`/`tracing` integration where applicable, and consistent conventions across every binding.

## Crates

| Crate                  | Version   | Status     | Tizen library               | Purpose                                          |
|------------------------|-----------|------------|-----------------------------|--------------------------------------------------|
| [`tizen`]              | `0.1.0`   | 🟢 alpha   | —                           | Umbrella; re-exports each binding behind a feature |
| [`tizen-dlog`]         | `0.1.0`   | 🟢 alpha   | `libdlog.so`                | `log` facade → Tizen `dlogutil`                  |
| [`tizen-app`]          | `0.1.0`   | 🟢 alpha   | `libcapi-appfw-application` | App lifecycle + `app_control`, optional `tokio`  |
| `tizen-system-info`    | —         | 📝 planned | `libcapi-system-info`       | Device capabilities, OS metadata                 |
| `tizen-sensor`         | —         | 📝 planned | `libcapi-system-sensor`     | Accelerometer, gyro, light, …                    |
| `tizen-bundle`         | —         | 📝 planned | `libbundle`                 | Bundle (key-value) IPC payloads                  |
| `tizen-notification`   | —         | 📝 planned | `libnotification`           | System notifications                             |

**Status legend:** 📝 planned · 🟡 in progress · 🟢 alpha · 🔵 beta · ✅ stable

Each safe wrapper has a companion `*-sys` crate
([`tizen-dlog-sys`](./crates/tizen-dlog-sys),
[`tizen-app-sys`](./crates/tizen-app-sys)) holding the raw `extern "C"`
declarations. End users don't depend on them directly.

[`tizen`]: ./crates/tizen
[`tizen-dlog`]: ./crates/tizen-dlog
[`tizen-app`]: ./crates/tizen-app

## Quick start

Two equivalent paths — pick whichever suits your project:

### Via the umbrella (recommended)

```toml
[dependencies]
tizen = { version = "0.1", features = ["dlog"] }
log = "0.4"
```

```rust
fn main() {
    tizen::dlog::init("MyApp").expect("install logger");
    log::info!("hello from rust");
    log::error!(target: "Network", "boom: {}", 42);
}
```

### Direct dependency

```toml
[dependencies]
tizen-dlog = "0.1"
log = "0.4"
```

```rust
fn main() {
    tizen_dlog::init("MyApp").expect("install logger");
    log::info!("hello from rust");
}
```

Both compile to the same code — the umbrella is a thin re-exporter. Use the
umbrella when you want one dependency that grows with you; use direct deps
when you want your `Cargo.toml` to spell out exactly which subsystems you touch.

On a Tizen device, view the output with `dlogutil MyApp:* Network:* '*:S'`.

## Building for Tizen

Use [`cargo-tizen`](https://github.com/Tizen-AIOS/cargo-tizen) — it
provisions the sysroot, sets up the cross toolchain, and injects the
`cfg(tizen)` flag this workspace's crates gate on. See
[`docs/tizen-target-setup.md`](./docs/tizen-target-setup.md).

Quick version:

```bash
cargo tizen build -A armv7l --release   # or: -A aarch64
sdb push target/tizen/<arch>/cargo/<rust-triple>/release/<your-binary> /opt/usr/
sdb shell /opt/usr/<your-binary>
```

End-to-end consumer examples (each is a standalone crate that depends on
this repo via git and opts into specific features):

- [`examples/hello-dlog/`](./examples/hello-dlog/) — `dlog` feature.
- [`examples/tizen-ui-app/`](./examples/tizen-ui-app/) — `app` feature,
  sync UI [`Lifecycle`](./crates/tizen-app/src/lifecycle.rs)
  (`ui_app_main`).
- [`examples/tizen-ui-app-tokio/`](./examples/tizen-ui-app-tokio/) —
  `app-tokio`, async `AsyncLifecycle` driven by `run_async_with` (custom
  tokio runtime config).
- [`examples/tizen-service-app/`](./examples/tizen-service-app/) — `app`
  feature, sync [`ServiceLifecycle`](./crates/tizen-app/src/service.rs)
  (`service_app_main`, headless).
- [`examples/tizen-service-app-tokio/`](./examples/tizen-service-app-tokio/)
  — `app-tokio`, async `AsyncServiceLifecycle` driven by
  `run_service_async_with` (headless + tokio runtime).

## Project layout

```
rust-tizen/
├── Cargo.toml                  # workspace manifest
├── crates/                     # all sub-crates
│   ├── tizen/                  # umbrella; re-exports bindings via cargo features
│   ├── tizen-dlog-sys/         # raw FFI for libdlog
│   └── tizen-dlog/             # safe wrapper + log::Log impl
├── examples/
│   └── hello-dlog/             # standalone consumer template (git dep)
├── docs/
│   ├── adding-a-binding.md     # how to contribute a new tizen-foo crate
│   └── tizen-target-setup.md   # cross-compile setup
├── scripts/
│   └── precommit.sh            # cargo fmt + check + clippy + test
├── AGENTS.md                   # entry point for coding agents (Codex / Claude)
├── CLAUDE.md                   # pointer to AGENTS.md
├── CONTRIBUTING.md
└── README.md
```

## Working with coding agents

Codex and Claude Code both read [`AGENTS.md`](./AGENTS.md) at the repo
root for build commands, layout, and Tizen-specific invariants
(`cfg(tizen)`, dlog FFI, `/opt/usr` deploy path, …). Keep that file in
sync when build commands or workflows change.

Every binding follows the same shape:

- `tizen-foo-sys/` — hand-written `extern "C"` declarations, `#![no_std]`, no deps beyond `core`.
- `tizen-foo/` — safe wrapper, idiomatic Rust API, optional `log`/`tracing` integration.
- One line added to `tizen/Cargo.toml` (a feature) and one line added to `tizen/src/lib.rs` (a re-export) so the umbrella picks it up.

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
