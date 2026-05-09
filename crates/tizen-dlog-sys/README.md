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

When the cargo target triple has `target_vendor = "tizen"` (e.g. `armv7l-tizen-linux-gnueabi`,
`aarch64-tizen-linux-gnu`), this crate dynamically links `libdlog.so`. On other
targets the FFI declarations exist but the symbols stay unresolved — you can
`cargo check` and run host-side unit tests, but a binary that actually calls
into dlog will fail to link off-device.

See [`docs/tizen-target-setup.md`](../../docs/tizen-target-setup.md) for sysroot setup.

## License

Dual-licensed under Apache-2.0 OR MIT, at your option. See the workspace root
for license texts.

Part of the [rust-tizen](../../README.md) project.
