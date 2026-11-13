#!/usr/bin/env bash
# peregrin release — thin shim over tarsi-serve (the org release plane,
# project ADR-0005; repo config: ./tarsi.toml). One call runs
# plan → changelog → bump → tag → push; publish targets are off
# (ADR-0004: crates.io is tamias-only, GitHub Releases unused).
# Operator-run on main — `tarsi-serve release` is dry-run by
# default; --yes is the explicit opt-in.
#
# crates.io publish is GONE: the old per-crate `cargo publish`
# loop (and its hypha → sepia → tamias → peregrin registry ordering
# constraint) is retired; every peregrin crate is internal — the
# `peregrin` command ships as the tamias pack in `libraries/peregrin`
# (`tamias install --allow-drivers ./libraries/peregrin`), with
# `peregrin-node`/`peregrin-hub` as tools/bin payload bins.
# `peregrin-testkit` is publish = false.
#
# The org release campaign (`libraries/release`, a peregrin bundle)
# drives this same `tarsi-serve release --yes` op on pushes — this
# script is the operator-run equivalent.
#
# Pack-ref sync (`[release.sync_refs]` in tarsi.toml) rewrites the
# product pack manifest + README install ref inside the release
# commit, so the tagged tree carries consistent versions (the other
# libraries/* packs version independently — untouched). The pack
# publish itself stays manual (`cargo xtask pack` stages the
# payload, then `tamias publish libraries/peregrin`).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
# worktree-local env (gitignored); the service env file is canonical
[ -f ./.env ] && { set -a; source ./.env; set +a; }

tarsi-serve release --dir . --yes

echo "released: see tags peregrin-*-v<ver>"
echo "next: cargo xtask pack && tamias publish libraries/peregrin  # ship 9aia:peregrin@<ver> to the org registry"
