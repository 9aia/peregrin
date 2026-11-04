#!/usr/bin/env bash
# bupha gate for v2 — runs in a detached worktree of the pushed sha.
# check = web markup gate + fmt + clippy -D warnings; test = spawned-bin
# prebuild + `cargo nextest run --workspace` (plain cargo test when
# nextest is absent).
set -euo pipefail

# The bupha watcher unit runs with the minimal systemd PATH
# (/usr/local/bin:/usr/bin) whenever it starts before the session env is
# imported — pull the user tool dirs in ourselves so cargo, nextest,
# and the installed sepia binaries resolve like in a login shell.
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

# Deliberately no CARGO_TARGET_DIR override: `xtask test`'s bin staging
# links peregrin-* builds out of `target/debug` (plus the shared-target
# fallback only) — a custom dir would leave spawned-binary tests
# resolving stale installed bins off PATH. The detached worktree is
# already job-scoped, so `target/` is isolated anyway.

# No sibling ../sepia in the detached worktree: deps come from the
# committed ssh:// git deps (Cargo.lock-pinned revs, already in the
# cargo git cache), and `sepia`/`sepia-driver-*` binaries that tests
# spawn resolve off PATH (~/.local/bin — kept current by `cargo xtask
# install` in the sepia checkout).

cargo xtask check
cargo xtask test
