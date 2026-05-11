# Contributing to rust-tizen

Thanks for your interest. This project is in its early days — we're actively
looking for contributors who can wrap additional Tizen native APIs in idiomatic
Rust crates.

## Ground rules

- Every binding wraps exactly **one** Tizen native library and lives in **two**
  crates: `tizen-foo-sys` (raw FFI) + `tizen-foo` (safe wrapper).
- Hand-written FFI bindings — no `bindgen` build dependency unless the API
  surface is genuinely too large to maintain by hand (>200 functions).
- `#![no_std]` for `-sys` crates. The safe wrappers may use `std`.
- Apache-2.0 OR MIT dual license. Contributions are accepted under the same.
- Conventional commits (`feat:`, `fix:`, `docs:`, `chore:`) for commit messages.
  Not strictly enforced but appreciated.

## Adding a new binding

The full step-by-step recipe is in [`docs/adding-a-binding.md`](./docs/adding-a-binding.md).
Short version:

1. **File an issue first** describing the Tizen library you want to wrap and
   the rough API surface. This avoids duplicate work and lets us discuss naming
   and scope.
2. **Create `crates/tizen-foo-sys/`** with hand-written `extern "C"` declarations.
3. **Create `crates/tizen-foo/`** with the safe Rust API.
4. **Add an example** under `crates/tizen-foo/examples/` gated on `cfg(tizen)`.
5. **Update the status table** in the root [`README.md`](./README.md).
6. **Open a PR** with on-device verification logs.

## Pull request requirements

Every PR must:

- [ ] Pass `./scripts/precommit.sh` (runs `cargo fmt --check`,
      `cargo check`, `cargo clippy -- -D warnings`, and `cargo test`).
- [ ] Include unit tests for any pure logic (level/error mapping, parsing,
      tag resolution, etc.) that runs on the host without a device.
- [ ] Document any unsafe block with a `// SAFETY:` comment explaining why the
      preconditions hold.
- [ ] Update the relevant `README.md` (workspace and/or per-crate) when adding
      public API.
- [ ] **For new bindings:** include a screenshot or transcript of `dlogutil`
      output (or equivalent) showing the binding works on a real Tizen device.
      State the Tizen version you tested against.

## Code style

- `cargo fmt` (default config) is enforced.
- `clippy` warnings are treated as errors in CI.
- Prefer `#[link]` attributes over `build.rs` whenever possible. `build.rs`
  should only appear when truly needed (codegen, dynamic config detection).
- Public API: every public item gets a doc comment explaining what it does and
  any non-obvious behaviour (panics, allocations, thread safety).
- Errors: prefer typed errors (`thiserror` is fine) over `anyhow` in libraries.
- Allocate sparingly on hot paths — log/event handlers run often.

## Testing

- **Unit tests** (host): inline `#[cfg(test)] mod tests { ... }` for pure logic.
  These run on every `cargo test`.
- **Integration tests** (on-device): provide an `examples/<feature>.rs` and
  document the on-device verification command in the example's doc comment.
  CI does not run on-device tests yet — verification is part of the PR review.

## Versioning and releases

All crates currently ship at the same workspace version (`0.1.x`). Release
process:

1. Bump `version` in `Cargo.toml`.
2. Update `CHANGELOG.md` (per crate, when we add them).
3. Tag the release: `vX.Y.Z`.
4. Maintainers run `cargo publish` for each crate in dependency order
   (`tizen-foo-sys` before `tizen-foo`).

## Getting unstuck

- Open an issue with the `question` label.
- For sensitive issues (security, conduct), email the maintainers (see Cargo.toml
  `authors` for contact). Public security disclosures should go through GitHub's
  security advisory feature.

## Code of conduct

We follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).
Be excellent to each other.
