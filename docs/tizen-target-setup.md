# Tizen target setup

This guide covers configuring `rustc` + `cargo` to cross-compile Rust code for
a Tizen device. The setup is one-time per build machine.

## Prerequisites

- A Tizen sysroot (rootstrap). Two common sources:
  - **Tizen Studio**: Install from <https://developer.tizen.org/development/tizen-studio/download>.
    Sysroots land under `~/tizen-studio/platforms/tizen-X.Y/tizen/rootstraps/`.
  - **GBS**: `sudo gbs createrepo` then `gbs build --rootstrap` to build one locally.
- A cross GCC toolchain. Tizen Studio ships these under
  `~/tizen-studio/tools/arm-linux-gnueabi-gcc-X.Y/` and equivalents for aarch64.
- (For deploying) `sdb` (Smart Development Bridge) for pushing binaries to a device.

## Adding a Rust target

Builds use the standard Linux ARM triples — Tizen-specific behaviour is
driven by the `cfg(tizen)` flag that `cargo tizen build` injects, not by the
target triple.

```sh
rustup target add armv7-unknown-linux-gnueabihf   # 32-bit ARM hard-float
rustup target add aarch64-unknown-linux-gnu       # 64-bit ARM
```

`cargo tizen build` sets `cfg(tizen)`, points the linker at the rootstrap
sysroot, and adds `usr/lib64` to the search path — so `tizen-*-sys` crates
that gate their `link(name = "...")` directive on `cfg(tizen)` pick up the
right libraries automatically.

## Building

```sh
cargo tizen build -A armv7l --release   # or: -A aarch64
```

The output binary lands at
`target/tizen/<arch>/cargo/<rust-triple>/release/<your-binary>`.

## Deploying to a device

```sh
sdb push target/tizen/<arch>/cargo/<rust-triple>/release/<your-binary> /opt/usr/
sdb shell chmod +x /opt/usr/<your-binary>
sdb shell /opt/usr/<your-binary>
```

Use `/opt/usr/` (or any path on a non-`noexec` mount) — `/tmp` is mounted
`noexec` on Tizen and will reject the executable.

## Viewing logs from a Rust app using `tizen-dlog`

```sh
sdb shell dlogutil <YourTag>:* '*:S'
```

`<YourTag>` is whatever you passed to `tizen_dlog::init("YourTag")`.

## Troubleshooting

**`cannot find -ldlog`** — your rootstrap is missing libdlog, or you're not
using `cargo tizen build` so `cfg(tizen)` never fires.

**Binary runs but produces no `dlogutil` output** — `dlogutil` filters out
DEBUG and below by default; use `dlogutil *:V` to see everything.

**`cannot execute binary file: Exec format error`** — wrong target arch.
32-bit Tizen wearables use `armv7l`; 64-bit phones/TVs use `aarch64`. Check
the device with `sdb shell uname -m`.
