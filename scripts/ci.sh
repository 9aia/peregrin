#!/usr/bin/env bash
# ci-gate campaign gate for v2 — runs in a detached worktree of the
# pushed sha (bupha watch is retired; the peregrin ci-gate campaign
# is the gate).
# check = fmt + clippy -D warnings; test = spawned-bin
# prebuild + `cargo nextest run --workspace` (plain cargo test when
# nextest is absent).
set -euo pipefail

export RUSTC_WRAPPER="${RUSTC_WRAPPER:-sccache}"
export SCCACHE_CONF="${SCCACHE_CONF:-file://$HOME/.cache/sccache}"

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

# Docs contract — cypera check + docs/ drift gate. Materializes a
# detached docs-branch worktree when the run tree lacks one (bupha
# gate runs), so it is hermetic in CI.
bash scripts/docs-check.sh

# --- mimus standard: org conformance leg (9aia-standard pack) ------
# Specs + runner ship in the 9aia-standard tamias pack; `tamias
# install` is a cache hit after the first fetch. Missing tooling
# skips with a warning; fail-severity spec violations fail the gate.
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
if command -v tamias >/dev/null 2>&1 && command -v mimus >/dev/null 2>&1; then
  _std_label="$(tamias install --no-bins 'git:github.com/9aia/workspace//libraries/9aia-standard' 2>/dev/null | sed -n 's/^installed \([^ ]*\).*/\1/p' | tail -1)"
  _std_dir=""
  [ -n "$_std_label" ] && _std_dir="$(tamias info "$_std_label" --json 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin).get("dir",""))' 2>/dev/null || true)"
  # Offline rerun fallback: whatever single install already exists.
  [ -z "$_std_dir" ] && _std_dir="$(tamias info 9aia-standard --json 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin).get("dir",""))' 2>/dev/null || true)"
  if [ -n "$_std_dir" ] && [ -f "$_std_dir/ci-standard.sh" ]; then
    bash "$_std_dir/ci-standard.sh"
  else
    echo "ci-standard: 9aia-standard pack unavailable — leg skipped" >&2
  fi
else
  echo "ci-standard: tamias/mimus not on PATH — leg skipped" >&2
fi
