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

## Tizen-specific invariants

- **Activation flag is `cfg(tizen)`.** Gate Tizen-only `link(...)`
  directives on it (`#[cfg_attr(tizen, link(name = "dlog", kind = "dylib"))]`).
  Do not use `cfg(target_vendor = "tizen")`; it's been removed everywhere.
- **`-sys` crates are `#![no_std]`** with no deps beyond `core::ffi`.
- **dlog FFI:** `dlog_print`, `__dlog_print` (private symbol; lets you pick
  `log_id_t`), `dlog_set_minimum_priority`. `dlog_print_raw` is a header
  macro on the C side, not an exported symbol — do not declare it.
- **64-bit Tizen libraries live in `usr/lib64`.** cargo-tizen passes the
  matching `-L` flag automatically.
- **`/tmp` is mounted `noexec` on Tizen.** Deploy binaries to
  `/opt/usr/<name>` (or another exec-allowed mount) before running.

## Adding a new binding

See [`docs/adding-a-binding.md`](docs/adding-a-binding.md). Short version:

1. Open a tracking issue.
2. Create `crates/tizen-foo-sys/` (raw FFI) and `crates/tizen-foo/` (safe wrapper).
3. Wire the umbrella: feature in `crates/tizen/Cargo.toml`, re-export in
   `crates/tizen/src/lib.rs`, workspace dep in root `Cargo.toml`.
4. Add a status-table row in [`README.md`](README.md).
5. Add an in-crate example gated on `#[cfg(tizen)]`.
6. PR body must include the on-device transcript.

## Common pitfalls

- **Forgetting `#[cfg_attr(tizen, link(...))]`** on `-sys` extern blocks.
  Either host CI fails (no libfoo on host) or device builds drop the link.
- **Declaring header macros as FFI symbols.** Check `nm -D` against the
  actual `.so` in the rootstrap.
- **Calling C variadics with user-supplied format strings.** Pre-format in
  Rust and pass `c"%s"` as the format — see `tizen-dlog`.
- **Editing `examples/hello-dlog/` as if it were a workspace member.**
  It is excluded from the workspace on purpose; its dep on `tizen` is via
  `git`, not `path`, so it mirrors real downstream consumption.

## Definition of done

- `./scripts/precommit.sh` is green.
- New bindings have an on-device transcript pasted in the PR body.
- AGENTS.md is updated in the same patch if build commands, layout, or
  invariants change.
