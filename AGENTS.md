# AGENTS.md

Map for coding agents working on `rust-tizen`. Deeper docs live under
[`docs/`](docs/). [`README.md`](README.md) is the human-facing entry point;
this file is the agent-facing one and should stay concise.

## What this repo is

Monorepo of focused, single-purpose Rust crates wrapping Tizen platform C
APIs. Each binding is a `tizen-foo-sys` (raw `extern "C"`) + `tizen-foo`
(safe wrapper) pair, re-exported from the `tizen` umbrella behind a cargo
feature so downstream projects opt into just the subsystems they need.

`tizen-dlog` (logging via `libdlog.so`) is the inaugural binding. The rest
are listed in [`README.md`](README.md).

## Layout

| Path                       | Purpose                                                |
|----------------------------|--------------------------------------------------------|
| `crates/tizen/`            | Umbrella crate; feature-gated re-exports               |
| `crates/tizen-dlog/`       | Safe wrapper over `libdlog.so`                         |
| `crates/tizen-dlog-sys/`   | Raw FFI for `libdlog.so` (`#![no_std]`, no deps)       |
| `examples/hello-dlog/`     | Standalone integration template (own workspace, git dep) |
| `docs/`                    | Cross-compile setup, contribution guide                |
| `scripts/`                 | Validation entry points (`precommit.sh`)               |

## Commands (exact)

| Goal                         | Command                                                 |
|------------------------------|---------------------------------------------------------|
| Run all host-side gates      | `./scripts/precommit.sh`                                |
| Format check                 | `cargo fmt --all -- --check`                            |
| Format fix                   | `cargo fmt --all`                                       |
| Type check                   | `cargo check --workspace --all-targets`                 |
| Lint                         | `cargo clippy --workspace --all-targets -- -D warnings` |
| Unit + doc tests             | `cargo test --workspace`                                |
| Cross-build the example      | `cd examples/hello-dlog && cargo tizen build -A aarch64 --release` (or `-A armv7l`) |

`./scripts/precommit.sh` runs fmt-check + check + clippy + test. Run it
before pushing.

Cross-builds require [`cargo-tizen`](https://github.com/Tizen-AIOS/cargo-tizen),
which provisions the rootstrap, configures the linker, and injects
`--cfg tizen`.

## Workspace conventions

- **`cfg(tizen)` is the activation flag.** Gate every Tizen-only
  `#[link(...)]` directive on it
  (`#[cfg_attr(tizen, link(name = "dlog", kind = "dylib"))]`).
  cargo-tizen injects `--cfg tizen`; `cargo check` on the host does not.
  Do not use `cfg(target_vendor = "tizen")`.
- **`-sys` crates are `#![no_std]`** with no dependencies beyond `core`.
- **Safe wrappers must build off-target.** Gate every FFI call site on
  `cfg(tizen)` and provide a host fallback (no-op, stderr, or equivalent)
  in the `cfg(not(tizen))` branch. The public API must compile and link
  on plain Linux so downstream apps don't need cfg gates around
  `tizen::dlog::*` calls. See `crates/tizen-dlog/src/logger.rs` for the
  pattern.
- **`examples/hello-dlog/` is intentionally outside the workspace** and
  depends on `tizen` via `git`, not `path`, so it mirrors real downstream
  consumption. Don't add it to `[workspace.members]`.

Library-specific details (FFI symbol choice, on-device deploy paths,
rootstrap layout) live in the relevant crate or in `docs/`, not here.

## Adding a new binding

Follow [`docs/adding-a-binding.md`](docs/adding-a-binding.md). The PR must
include an on-device verification transcript.

## Definition of done

- `./scripts/precommit.sh` is green.
- New bindings have an on-device transcript pasted in the PR body.
- AGENTS.md is updated in the same patch if build commands, layout, or
  invariants change.
