# tizen-dlog

[![crates.io](https://img.shields.io/crates/v/tizen-dlog.svg)](https://crates.io/crates/tizen-dlog)
[![docs.rs](https://docs.rs/tizen-dlog/badge.svg)](https://docs.rs/tizen-dlog)

Idiomatic Rust logging into Tizen's `dlog` system, via the [`log`](https://crates.io/crates/log) facade.

```rust
fn main() {
    tizen_dlog::init("MyApp").expect("install logger");
    log::info!("hello from rust");
    log::error!(target: "Network", "boom: {}", 42);
}
```

On a Tizen device, view the output with:

```sh
dlogutil MyApp:* Network:* '*:S'
```

## Features

- Implements `log::Log`, so any code using `log::info!`/`error!`/etc. just works.
- Per-call tag override via `target: "MyTag"`, plus a global default tag.
- Configurable level filter, dlog buffer (`LogId::LOG_ID_MAIN` / `LOG_ID_APPS` / …),
  and tag-resolution strategy.
- Format-string injection safe: messages are pre-formatted in Rust and passed to
  `dlog_print` with a fixed `"%s"` specifier.
- Interior NUL bytes in user data are sanitised — logging never panics on input.
- Adds ~6 KB of code to a release binary; `libdlog.so` itself is dynamically linked.

## Builder

```rust
use log::LevelFilter;
use tizen_dlog::{DlogLogger, LogId, TagStrategy};

DlogLogger::builder()
    .default_tag("MyApp")
    .level(LevelFilter::Debug)
    .log_id(LogId::LOG_ID_APPS)
    .tag_strategy(TagStrategy::TargetThenDefault)
    .install()
    .unwrap();
```

## Level mapping

| `log::Level` | dlog priority   | dlogutil letter |
|--------------|-----------------|-----------------|
| `Error`      | `DLOG_ERROR`    | `E`             |
| `Warn`       | `DLOG_WARN`     | `W`             |
| `Info`       | `DLOG_INFO`     | `I`             |
| `Debug`      | `DLOG_DEBUG`    | `D`             |
| `Trace`      | `DLOG_VERBOSE`  | `V`             |

## Tag strategies

| Strategy             | Behaviour                                                                           |
|----------------------|-------------------------------------------------------------------------------------|
| `TargetThenDefault`  | Use `target` if it looks like an explicit override (no `::`); else default. Default. |
| `AlwaysDefault`      | Always use the configured default tag.                                              |
| `AlwaysTarget`       | Always use `record.target()` (module path when not overridden).                     |

## Building for Tizen

You need a Tizen sysroot configured for cross-compilation. See
[`docs/tizen-target-setup.md`](../../docs/tizen-target-setup.md) in the repo root.

## License

Dual-licensed under Apache-2.0 OR MIT, at your option. See the workspace root
for license texts.

Part of the [rust-tizen](../../README.md) project.
