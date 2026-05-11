#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

step() {
    printf '\n==> %s\n' "$*"
}

step 'cargo fmt --all -- --check'
cargo fmt --all -- --check

step 'cargo check --workspace --all-targets'
cargo check --workspace --all-targets

step 'cargo clippy --workspace --all-targets -- -D warnings'
cargo clippy --workspace --all-targets -- -D warnings

step 'cargo test --workspace'
cargo test --workspace

printf '\nprecommit: all checks passed\n'
