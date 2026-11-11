#!/usr/bin/env bash
# ci-gate campaign gate for v2 — runs in a detached worktree of the
# pushed sha (bupha watch is retired; the peregrin ci-gate campaign
# is the gate).
# check = fmt + clippy -D warnings; test = spawned-bin
# prebuild + `cargo nextest run --workspace` (plain cargo test when
# nextest is absent).
set -euo pipefail

# The gate may run with the minimal systemd PATH
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
# cargo git cache), and `xtask test`'s prebuild builds the spawned
# sepia binaries (`sepia-driver-*`, `sepia-mock-acp-agent`) inside
# that git checkout — the same target/debug sepia-testkit's
# resolution falls back to. The `sepia` command itself is a tamias
# shim post-T-012: `peregrin-session`'s PATH fallback resolves it on
# machines with the sepia pack installed; dev checkouts may still
# carry a stale `target/debug/sepia` artifact.

cargo xtask check
cargo xtask test

# Optional line-coverage gate (llvm-cov — slow, so off by default).
# Opt in per job with PEREGRIN_COVERAGE=1; scripts/coverage.sh scopes
# the 100% check to the Rust files that differ from the diff base.
if [ "${PEREGRIN_COVERAGE:-0}" = "1" ]; then
  bash scripts/coverage.sh
fi
