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

Tizen target triples are not built into rustup by default. Two options:

### Option A: Use the closest built-in target

For most cases, a generic Linux ARM target works fine because the vendor field
isn't load-bearing for most code:

```sh
rustup target add armv7-unknown-linux-gnueabihf   # 32-bit ARM hard-float
rustup target add aarch64-unknown-linux-gnu       # 64-bit ARM
```

The `target_vendor = "tizen"` cfg won't fire on these targets, so the
`#[cfg_attr(target_vendor = "tizen", link(...))]` directive in `tizen-*-sys`
crates won't request libdlog automatically. You'll need to add the link
directive yourself in your project's `.cargo/config.toml`:

```toml
[target.armv7-unknown-linux-gnueabihf]
rustflags = ["-C", "link-arg=-ldlog"]
```

### Option B: Use a custom Tizen target JSON (recommended)

Save the following as `armv7l-tizen-linux-gnueabi.json` somewhere convenient:

```json
{
  "arch": "arm",
  "data-layout": "e-m:e-p:32:32-Fi8-i64:64-v128:64:128-a:0:32-n32-S64",
  "env": "gnu",
  "features": "+v7,+thumb2,+vfp3,-d32,-fp16",
  "linker-flavor": "gcc",
  "llvm-target": "armv7-unknown-linux-gnueabi",
  "max-atomic-width": 64,
  "os": "linux",
  "panic-strategy": "abort",
  "target-pointer-width": "32",
  "vendor": "tizen"
}
```

Then build with:

```sh
cargo +nightly build -Z build-std=std,panic_abort \
    --target ./armv7l-tizen-linux-gnueabi.json
```

This is the only way to get `target_vendor = "tizen"` to fire, which means the
auto-link directives in `tizen-*-sys` crates will work without manual config.
Requires nightly Rust for `-Z build-std`.

## `.cargo/config.toml`

Configure the linker, sysroot, and library search path. Example for Tizen
Studio installed in `~/tizen-studio`:

```toml
[target.armv7l-tizen-linux-gnueabi]
linker = "/Users/you/tizen-studio/tools/arm-linux-gnueabi-gcc-9.2/bin/arm-linux-gnueabi-gcc"
rustflags = [
    "-C", "link-arg=--sysroot=/Users/you/tizen-studio/platforms/tizen-7.0/tizen/rootstraps/tizen-7.0-device.core",
    "-L", "/Users/you/tizen-studio/platforms/tizen-7.0/tizen/rootstraps/tizen-7.0-device.core/usr/lib",
]

[target.aarch64-tizen-linux-gnu]
linker = "/Users/you/tizen-studio/tools/aarch64-linux-gnu-gcc-9.2/bin/aarch64-linux-gnu-gcc"
rustflags = [
    "-C", "link-arg=--sysroot=/Users/you/tizen-studio/platforms/tizen-7.0/tizen/rootstraps/tizen-7.0-device-64.core",
    "-L", "/Users/you/tizen-studio/platforms/tizen-7.0/tizen/rootstraps/tizen-7.0-device-64.core/usr/lib",
]
```

Adjust paths for your installation. Replace `tizen-7.0` with your target Tizen
version.

## Building

```sh
cargo build --release --target armv7l-tizen-linux-gnueabi
```

The output binary lands at `target/armv7l-tizen-linux-gnueabi/release/<your-binary>`.

## Deploying to a device

```sh
sdb push target/armv7l-tizen-linux-gnueabi/release/<your-binary> /tmp/
sdb shell chmod +x /tmp/<your-binary>
sdb shell /tmp/<your-binary>
```

## Viewing logs from a Rust app using `tizen-dlog`

```sh
sdb shell dlogutil <YourTag>:* '*:S'
```

`<YourTag>` is whatever you passed to `tizen_dlog::init("YourTag")`.

## Stripping for size

```sh
arm-linux-gnueabi-strip target/armv7l-tizen-linux-gnueabi/release/<your-binary>
```

The release profile in this workspace already enables `strip = "debuginfo"` and
`lto = "thin"` — usually enough. Run `arm-linux-gnueabi-size <binary>` to
inspect section sizes.

## Troubleshooting

**`error: linking with cc failed`** — usually the linker can't find `libdlog.so`
or your other native lib. Check the `-L` path in `.cargo/config.toml` points
at the correct rootstrap's `usr/lib` directory.

**`undefined reference to dlog_print_raw`** — the `-ldlog` flag isn't being
passed. If using Option A above, make sure you added `link-arg=-ldlog` to your
project's `rustflags`. If using Option B, make sure your custom target JSON
sets `"vendor": "tizen"`.

**Binary runs but produces no `dlogutil` output** — check the priority filter:
`dlogutil *:V` shows everything. Default `dlogutil` filters out DEBUG and below.

**`cannot execute binary file: Exec format error`** — wrong target architecture
for the device. 32-bit Tizen wearables use `armv7l-tizen-linux-gnueabi`,
64-bit phones/TVs use `aarch64-tizen-linux-gnu`. Check the device with
`sdb shell uname -m`.
