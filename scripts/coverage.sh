#!/usr/bin/env bash
# Diff-scoped line-coverage gate — 100% on the crates a change touches
# (PL-003's standing gate: "100% line coverage on all *new* code").
#
# The old whole-workspace form (`grep DA:,0 lcov.info`) failed on
# pre-existing gaps unrelated to the change, so it gated nothing in
# practice. This scopes the check to the Rust files that differ from a
# base ref.
#
#   PEREGRIN_COVERAGE_BASE=<ref>   diff base (default: merge-base with
#                                  origin/main; if neither resolves,
#                                  gate the whole workspace)
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
lcov="$work/lcov.info"

cargo xtask coverage --lcov --output-path "$lcov"

base="${PEREGRIN_COVERAGE_BASE:-}"
if [ -z "$base" ] && git rev-parse --verify -q origin/main >/dev/null 2>&1; then
  base="$(git merge-base HEAD origin/main 2>/dev/null || true)"
fi

if [ -z "$base" ]; then
  echo "coverage: no diff base resolved — gating the whole workspace"
  awk '/^SF:/ { print substr($0, 4) }' "$lcov" > "$work/want"
else
  echo "coverage: gating Rust files touched since ${base:0:12}"
  git diff --name-only "$base"...HEAD -- 'crates/**/*.rs' > "$work/want"
fi

# One lcov pass: report every DA:,0 line whose file is in the want-set.
# `SF:` paths are absolute; normalise against the repo root before
# comparing, and tolerate a relative form too.
awk -v list="$work/want" -v root="$root/" '
  BEGIN { while ((getline line < list) > 0) if (line != "") want[line] = 1 }
  /^SF:/ {
    cur = substr($0, 4)
    sub("^" root, "", cur)
    sub("^\\./", "", cur)
  }
  cur in want && /^DA:[0-9]+,0(,|$)/ { print cur }
' "$lcov" > "$work/hits" || true

if [ -s "$work/hits" ]; then
  echo "coverage: uncovered lines remain in touched files:"
  sort -u "$work/hits" | sed 's/^/  /'
  exit 1
fi
echo "coverage: 100% line coverage on touched files"
