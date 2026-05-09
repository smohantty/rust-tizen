# tizen-dlog-sys

[![crates.io](https://img.shields.io/crates/v/tizen-dlog-sys.svg)](https://crates.io/crates/tizen-dlog-sys)
[![docs.rs](https://docs.rs/tizen-dlog-sys/badge.svg)](https://docs.rs/tizen-dlog-sys)

Raw FFI bindings to Tizen's [`libdlog`](https://docs.tizen.org/), mirroring `<dlog.h>`.

`#![no_std]`, no dependencies, hand-written `extern "C"` declarations. About 30 lines
of bindings.

For a safe, idiomatic Rust API on top of these bindings, use
[`tizen-dlog`](../tizen-dlog).

## What's exposed

```rust
pub enum log_priority { DLOG_UNKNOWN, DLOG_DEFAULT, DLOG_VERBOSE, DLOG_DEBUG,
                        DLOG_INFO, DLOG_WARN, DLOG_ERROR, DLOG_FATAL, DLOG_SILENT }

pub enum log_id_t { LOG_ID_INVALID, LOG_ID_MAIN, LOG_ID_RADIO, LOG_ID_SYSTEM,
                    LOG_ID_APPS, LOG_ID_KMSG, LOG_ID_SYSLOG }

extern "C" {
    pub fn dlog_print(prio, tag, fmt, ...) -> c_int;
    pub fn dlog_print_raw(log_id, prio, tag, fmt, ...) -> c_int;
}
```

## Linking

This crate dynamically links `libdlog.so` when **either** activation fires:

- `--cfg tizen` is set (the path used by [`cargo-tizen`](https://github.com/smohantty/cargo-tizen),
  which injects it via `RUSTFLAGS` for every `cargo tizen build`), **or**
- the `tizen` cargo feature is enabled (manual opt-in for users not going
  through cargo-tizen).

On non-Tizen builds neither activates and the FFI declarations stay unresolved
in the rlib — you can run `cargo check` and host-side unit tests fine. A binary
that actually calls into dlog will fail to link off-device, which is correct.

rustc forbids overriding the built-in `target_vendor` cfg via `--cfg`
(`explicit_builtin_cfgs_in_flags` hard error), so a free-form custom cfg is
the only way for build tooling to signal "this build targets Tizen" without
forcing a custom target JSON + nightly Rust.

See [`docs/tizen-target-setup.md`](../../docs/tizen-target-setup.md) for the
full activation matrix.

## License

Dual-licensed under Apache-2.0 OR MIT, at your option. See the workspace root
for license texts.

Part of the [rust-tizen](../../README.md) project.
