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
# Preserved repo step: the product pack manifest + README install
# ref are synced to the new version *before* the release commit
# lands, so the tagged tree carries consistent versions (the other
# libraries/* packs version independently — untouched). The pack
# publish itself stays manual (`cargo xtask pack` stages the
# payload, then `tamias publish libraries/peregrin`).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
# worktree-local env (gitignored); the service env file is canonical
[ -f ./.env ] && { set -a; source ./.env; set +a; }

ws_version() { grep -m1 '^version = "' Cargo.toml | cut -d'"' -f2; }

# Learn the version tarsi will bump to (all releasing units share one
# workspace version — empty output means "don't sync").
new="$(tarsi-serve plan --dir . | python3 -c '
import json,sys
d=json.load(sys.stdin)
if not d.get("ok"): sys.exit(0)
units=[u for u in d["result"]["plan"]["units"] if u.get("releases")]
vs={u["new_version"] for u in units}
print(vs.pop() if len(vs)==1 else "")
')"
old="$(ws_version)"
if [ -n "$new" ] && [ "$new" != "$old" ]; then
    sed -i "s/^version: \"${old}\"/version: \"${new}\"/" libraries/peregrin/tamias.yaml
    sed -i "s|peregrin@${old}|peregrin@${new}|g" README.md
    git add libraries/peregrin/tamias.yaml README.md
    git commit -qm "chore(release): sync peregrin@${new} version refs ahead of tarsi bump"
fi

tarsi-serve release --dir . --yes

echo "released: peregrin ${new:-$old} (tag peregrin-v${new:-$old}, per-unit tags for the rest)"
echo "next: cargo xtask pack && tamias publish libraries/peregrin  # ship 9aia:peregrin@${new:-$old} to the org registry"
