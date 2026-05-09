# Tizen target setup

This guide covers cross-compiling Rust code for a Tizen device. The
**recommended path is [`cargo-tizen`](https://github.com/smohantty/cargo-tizen)** —
it manages the sysroot, linker, packaging (RPM/TPK), and device install with one
command. Manual setup is also documented below for users who can't use cargo-tizen.

## How rust-tizen knows it's targeting Tizen

The `tizen-*-sys` crates include link directives gated on:

```rust
#[cfg_attr(any(tizen, feature = "tizen"), link(name = "dlog", kind = "dylib"))]
```

`tizen` here is a **custom free-form cfg flag**, not the built-in `target_vendor`.
rustc forbids overriding `target_vendor` via `--cfg` (`explicit_builtin_cfgs_in_flags`
hard error), so a custom cfg is the only way for build tooling to signal "this
build targets Tizen" without forcing users onto a custom target JSON + nightly.

There are two activation paths:

| Path | How `tizen` cfg is set | When to use |
|---|---|---|
| **`cargo-tizen`** (recommended) | Tool injects `--cfg tizen` via `RUSTFLAGS` automatically | Any normal build |
| **Cargo feature `tizen`** | User adds `features = ["tizen"]` to their dep | Manual cross-compile without cargo-tizen |

## Path 1: Build with cargo-tizen (recommended)

Install:

```sh
cargo install cargo-tizen   # or: cargo install --git https://github.com/smohantty/cargo-tizen
cargo tizen doctor          # verify SDK / sysroot
```

In your project's `Cargo.toml`:

```toml
[dependencies]
tizen = { version = "0.1", features = ["dlog"] }
log = "0.4"
```

Build, package, and install:

```sh
cargo tizen build   -A armv7l --release
cargo tizen tpk     -A armv7l --release
cargo tizen install -A armv7l --release
```

cargo-tizen sets `--cfg tizen` for you, so the link directive activates and
`libdlog.so` gets linked from the rootstrap. No extra config needed in your
`Cargo.toml` or `.cargo/config.toml`.

## Path 2: Manual cross-compile (no cargo-tizen)

If you can't use cargo-tizen — for instance in a constrained CI environment —
you have two sub-options:

### 2a: Enable the `tizen` cargo feature

Add `features = ["tizen"]` to your dependency on `tizen-dlog` (or on the
umbrella `tizen` crate):

```toml
[dependencies]
tizen-dlog = { version = "0.1", features = ["tizen"] }
log = "0.4"
```

Then configure linker/sysroot in your `.cargo/config.toml`:

```toml
[target.armv7-unknown-linux-gnueabi]
linker = "/path/to/tizen-studio/tools/arm-linux-gnueabi-gcc-9.2/bin/arm-linux-gnueabi-gcc"
rustflags = [
    "-C", "link-arg=--sysroot=/path/to/tizen-rootstrap",
    "-L", "/path/to/tizen-rootstrap/usr/lib",
]

[target.aarch64-unknown-linux-gnu]
linker = "/path/to/tizen-studio/tools/aarch64-linux-gnu-gcc-9.2/bin/aarch64-linux-gnu-gcc"
rustflags = [
    "-C", "link-arg=--sysroot=/path/to/tizen-rootstrap-64",
    "-L", "/path/to/tizen-rootstrap-64/usr/lib",
]
```

Build:

```sh
rustup target add armv7-unknown-linux-gnueabi
cargo build --release --target armv7-unknown-linux-gnueabi
```

### 2b: Set `--cfg tizen` yourself via RUSTFLAGS

Equivalent to 2a but uses the cfg flag directly instead of the cargo feature:

```toml
[target.armv7-unknown-linux-gnueabi]
linker = "..."
rustflags = [
    "--cfg", "tizen",
    "-C", "link-arg=--sysroot=...",
    "-L", "/path/to/tizen-rootstrap/usr/lib",
]
```

Then:

```sh
cargo build --release --target armv7-unknown-linux-gnueabi
```

Pick whichever you prefer — they activate the same code path.

## Path 3: Custom target JSON (advanced, nightly only)

If you want the *real* `target_vendor = "tizen"` (e.g. to share build
infrastructure with C/C++ Tizen projects that already key on `tizen-linux-gnueabi`
triples), you can write a custom target spec:

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

But: rustup ships no precompiled std for this triple, so you'd need
`-Z build-std` (nightly only). For most users, paths 1 and 2 are simpler.

The `tizen` cfg activation (paths 1 and 2) gives you the same *behaviour* as a
real `target_vendor = "tizen"` would, on stable Rust, without rebuilding std.

## Deploying to a device

```sh
# via cargo-tizen
cargo tizen install -A armv7l --release

# manually
sdb push target/armv7-unknown-linux-gnueabi/release/<your-binary> /tmp/
sdb shell chmod +x /tmp/<your-binary>
sdb shell /tmp/<your-binary>
```

## Viewing logs from a Rust app using `tizen-dlog`

```sh
sdb shell dlogutil <YourTag>:* '*:S'
```

`<YourTag>` is whatever you passed to `tizen_dlog::init("YourTag")` (or
`tizen::dlog::init` via the umbrella).

## Stripping for size

```sh
arm-linux-gnueabi-strip target/armv7-unknown-linux-gnueabi/release/<your-binary>
```

The workspace's release profile already enables `strip = "debuginfo"` and
`lto = "thin"`. Run `arm-linux-gnueabi-size <binary>` for a section breakdown.

## Troubleshooting

**`error: linking with cc failed: undefined reference to dlog_print_raw`** —
the `tizen` cfg/feature isn't active so the link directive didn't fire. Either
build through cargo-tizen, enable `features = ["tizen"]` on your dep, or add
`--cfg tizen` to RUSTFLAGS.

**`error: linker cannot find -ldlog`** — the cfg/feature *is* active and the
link directive fired, but the linker can't find `libdlog.so` in the sysroot.
Check the `-L` path in `.cargo/config.toml` points at the rootstrap's
`usr/lib` directory.

**`error: cannot execute binary file: Exec format error`** on the device —
wrong target architecture. 32-bit Tizen wearables use armv7l; 64-bit phones/TVs
use aarch64. Check with `sdb shell uname -m`.
