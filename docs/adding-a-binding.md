# Adding a new Tizen binding

This document is the recipe for wrapping a Tizen native library as a pair of
Rust crates (`tizen-foo-sys` + `tizen-foo`). It exists so the project scales
beyond a handful of contributors — anyone who can hand-write `extern "C"` and
read a Tizen header should be able to ship a binding.

## Before you start

- **File an issue first.** Describe the Tizen library you want to wrap, the
  rough API surface, and naming. This catches duplicate work and lets us
  align on conventions.
- **Pick a stable Tizen API.** Prefer libraries marked stable in the Tizen
  Native API Reference. Avoid anything labelled experimental or version-locked
  to a single Tizen release.
- **Read an existing binding.** [`tizen-dlog`](../crates/tizen-dlog) is the
  reference implementation — copy its structure.

## The recipe

### 1. Identify the Tizen library

You need three things:
- Library name as the linker sees it: e.g. `dlog`, `capi-system-info`,
  `capi-appfw-application`. (The thing after `-l` when GCC links it. Drop the
  leading `lib` and the `.so` suffix.)
- Header file path inside the Tizen sysroot: e.g.
  `usr/include/dlog/dlog.h`, `usr/include/system/system_info.h`.
- The set of public types and functions you want to expose.

### 2. Create `crates/tizen-foo-sys/`

```
crates/tizen-foo-sys/
├── Cargo.toml
├── README.md
└── src/lib.rs
```

`Cargo.toml`:

```toml
[package]
name = "tizen-foo-sys"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
authors.workspace = true
description = "Raw FFI bindings to Tizen's libfoo"
categories = ["external-ffi-bindings", "no-std"]
keywords = ["tizen", "foo", "ffi", "bindings"]
readme = "README.md"

[lib]

[lints]
workspace = true
```

`src/lib.rs`:

```rust
//! Raw FFI bindings to Tizen's `libfoo`.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![no_std]

use core::ffi::{c_char, c_int};

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum foo_status_e {
    FOO_STATUS_OK = 0,
    FOO_STATUS_ERROR = -1,
}

#[cfg_attr(target_vendor = "tizen", link(name = "foo", kind = "dylib"))]
extern "C" {
    pub fn foo_init() -> foo_status_e;
    pub fn foo_get_value(out: *mut c_int) -> foo_status_e;
    // ...
}
```

**Rules for `-sys` crates:**
- `#![no_std]`. Use `core::ffi`, not `std::ffi`.
- `#[cfg_attr(target_vendor = "tizen", link(...))]` on the `extern "C"` block.
  This makes off-device `cargo check` work without a Tizen sysroot.
- No dependencies beyond `core`.
- No `build.rs` unless absolutely needed.
- Hand-write the bindings. If the API has more than ~200 functions, ask in the
  issue thread before adding `bindgen` — usually you can split the binding
  into focused sub-modules instead.

### 3. Create `crates/tizen-foo/`

```
crates/tizen-foo/
├── Cargo.toml
├── README.md
├── examples/hello_foo.rs
└── src/
    ├── lib.rs
    ├── error.rs
    └── ...
```

`Cargo.toml`:

```toml
[package]
name = "tizen-foo"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
authors.workspace = true
description = "Idiomatic Rust wrapper for Tizen's foo API"
categories = ["api-bindings"]
keywords = ["tizen", "foo"]
readme = "README.md"

[dependencies]
tizen-foo-sys = { workspace = true }

[lints]
workspace = true
```

Add `tizen-foo-sys = { version = "0.1.0", path = "crates/tizen-foo-sys" }` to
the workspace `[workspace.dependencies]` table.

**Rules for safe wrappers:**
- Convert C error codes to `Result<T, FooError>`.
- Convert raw `*const c_char` to `&str` / `String`, with explicit UTF-8 handling
  (don't panic on invalid UTF-8 — return an error or replacement chars).
- Convert raw `*mut T` out-params to function return values.
- Document thread safety. If the underlying C API is not thread-safe, mark the
  Rust handle `!Send` / `!Sync` accordingly.
- Document panic safety. Public API should not panic on user input.
- Prefer `RAII` for handles that need cleanup (`impl Drop`).
- Inline unit tests for any pure logic.

### 4. Add an example

```rust
// crates/tizen-foo/examples/hello_foo.rs

#[cfg(target_vendor = "tizen")]
fn main() {
    // real example here
}

#[cfg(not(target_vendor = "tizen"))]
fn main() {
    eprintln!("hello_foo: build for a Tizen target to actually exercise the API.");
}
```

The cfg gate lets the example compile (as a stub) on host so `cargo build --examples`
doesn't fail in CI without a Tizen sysroot.

Document the on-device test procedure in the example's doc comment:

```rust
//! On a Tizen device:
//!     cargo build --release --target armv7l-tizen-linux-gnueabi --example hello_foo
//!     sdb push target/.../hello_foo /tmp/
//!     sdb shell /tmp/hello_foo
//!     # expected output: ...
```

### 5. Wire the umbrella crate

In [`crates/tizen/Cargo.toml`](../crates/tizen/Cargo.toml):

```toml
[features]
foo = ["dep:tizen-foo"]   # add this line

[dependencies]
tizen-foo = { workspace = true, optional = true }   # add this line
```

In [`crates/tizen/src/lib.rs`](../crates/tizen/src/lib.rs):

```rust
#[cfg(feature = "foo")]
#[cfg_attr(docsrs, doc(cfg(feature = "foo")))]
pub use tizen_foo as foo;
```

Also add `tizen-foo = { version = "0.1.0", path = "crates/tizen-foo" }` to
the workspace `[workspace.dependencies]` table in the root `Cargo.toml`.

### 6. Update the status table

In the root [`README.md`](../README.md), add rows to the `## Crates` table:

```markdown
| `tizen-foo`        | `0.1.0` | 🟢 alpha | `libfoo` | One-line description |
| `tizen-foo-sys`    | `0.1.0` | 🟢 alpha | `libfoo` | Raw FFI bindings     |
```

…and update the `tizen` umbrella's feature list in its own README and rustdoc.

### 7. Run local checks

```sh
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

All four must pass before opening a PR.

### 8. Verify on a Tizen device

Cross-compile, push, run, capture output. Paste the transcript into the PR
description. State the Tizen version (e.g. "Tizen 7.0 on a Galaxy Watch 5").

```sh
cargo build --release --target armv7l-tizen-linux-gnueabi --example hello_foo
sdb push target/armv7l-tizen-linux-gnueabi/release/examples/hello_foo /tmp/
sdb shell /tmp/hello_foo
# (capture and paste the output)
```

### 9. Open the PR

Title: `Add tizen-foo: <one-line description>`

Body must include:
- Link to the tracking issue
- Tizen library version / API level supported
- Tizen device + OS version tested
- On-device output transcript
- Anything notable about the binding (unsafe blocks, thread-safety choices, etc.)

## Common pitfalls

- **Forgetting `#[cfg_attr(target_vendor = "tizen", link(...))]`.** Without it
  either CI fails (no libfoo on host) or device builds don't link libfoo.
- **Calling C variadics directly.** If you need to wrap a printf-style function,
  pre-format in Rust and pass `c"%s"` as the format. See `tizen-dlog`'s
  `dlog_print_raw` usage.
- **Holding C strings in a struct field.** A `*const c_char` returned from C
  may be invalidated when you call other C functions. Either copy to `String`
  immediately or document the lifetime constraint.
- **Assuming `i32` for enums.** Some Tizen enums are `int8_t` or `int16_t` in
  the actual ABI. Check the header's `__attribute__` annotations.
- **Skipping NUL sanitisation on user-supplied strings.** `CString::new` panics
  on interior NULs. Either sanitise (replace with space) or return an error.
- **Adding a `build.rs` you don't need.** `#[link]` and `[workspace.lints]`
  cover most cases. Ask before introducing `build.rs`.
