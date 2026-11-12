#!/usr/bin/env bash
# Run manually: `./scripts/release.sh` from the repo root (needs
# `.env` with CARGO_REGISTRY_TOKEN; dist + gh for binary artifacts).
# bupha is deprecated — no watch registration; releases are operator-run.
# The leading sleep is the debounce window: only the last invocation
# in a quiet window releases. Commits/tags are pushed to main.
#
# crates.io status (T-003): every internal dep carries a `version`
# req alongside its git source, so `cargo publish` resolves them
# against the registry — BUT only after the org publishes in order:
#   hypha 0.1.0 → sepia 0.1.0 → tamias (already publishing) → peregrin
# Until hypha-*/sepia-* 0.1.0 exist on crates.io the publish loop
# below fails on the first crate; the tag + binary-release half still
# works standalone (git tag peregrin-v<ver> && git push --tags).
# Needs: `cargo-edit` (cargo set-version), `cargo-dist` (dist), `gh`.
set -euo pipefail
# worktree-local secrets (gitignored); the service env file is canonical
[ -f ./.env ] && { set -a; source ./.env; set +a; }
sleep "${PEREGRIN_RELEASE_DEBOUNCE:-300}"

git fetch origin --tags -q
last="$(git describe --tags --abbrev=0 2>/dev/null || true)"
if [ -z "$last" ]; then
    echo "no tags yet — tag the first release manually to seed the flow"; exit 0
fi
changed="$(git diff --name-only "${last}..HEAD")"

# Publish order — peregrin-* crates, dependencies first (internal
# edges only; hypha/sepia/tamias deps are already versioned registry
# reqs). peregrin-testkit is publish = false — excluded.
order="peregrin-core peregrin-auth peregrin-session peregrin-hub \
       peregrin-store peregrin-git peregrin-loader peregrin-queue \
       peregrin-remote peregrin-template peregrin-config \
       peregrin-bus peregrin-config-toml peregrin-config-yaml \
       peregrin-config-file peregrin-loader-yaml peregrin-remote-http \
       peregrin-store-json peregrin-sink-github-status peregrin-driver \
       peregrin-driver-shell peregrin-driver-agent \
       peregrin-driver-council peregrin-driver-http \
       peregrin-driver-human peregrin-driver-git peregrin-engine \
       peregrin-http peregrin-driver-github-webhook \
       peregrin-driver-schedule peregrin-driver-sturn \
       peregrin-driver-subworkflow peregrin-node \
       peregrin-cli"

touched=""
for c in $order; do
    grep -q "^crates/$c/" <<<"$changed" && touched="$touched $c"
done
if [ -z "$touched" ]; then echo "no crate changes since $last"; exit 0; fi

published=""
tags=""
for c in $order; do
    grep -qw "$c" <<<"$touched" || continue
    cargo set-version -p "$c" --bump patch
    # crates.io rate-limits *new* crates (a time window per push); retry
    # through it rather than failing the release.
    for i in $(seq 1 60); do
        out="$(cargo publish -p "$c" --allow-dirty 2>&1)" && break
        if grep -q '429\|too many new crates' <<<"$out"; then
            echo "rate-limited on $c — sleeping 60s"; sleep 60
        elif grep -q 'already exists\|already uploaded' <<<"$out"; then
            break
        else
            echo "$out" | tail -3; sleep 30
        fi
    done
    published="$published $c"
    # Per-crate tag — versions diverge independently in a monorepo.
    ver_c="$(grep -m1 '^version' "crates/$c/Cargo.toml" | cut -d'"' -f2)"
    tags="$tags $c-v$ver_c"
done

git commit -aqm "release: bump crate versions" || true
for t in $tags; do git tag "$t"; done
ver="$(grep -m1 '^version' crates/peregrin-cli/Cargo.toml | cut -d'"' -f2)"
git push -q origin HEAD:main --tags

# binaries: releases are manual — GHA workflows were removed org-wide.
# This script bumps, publishes, and tags; the `dist build` path below
# produces release tarballs (`peregrin-<target>.tar.gz` — the name
# install-peregrin probes). To ship:
#   git tag peregrin-v${ver} && git push origin peregrin-v${ver}
if command -v dist >/dev/null && command -v gh >/dev/null; then
    dist build --output-format=json > /tmp/dist-manifest.json 2>&1 || true
    for f in target/distrib/*; do
        [ -f "$f" ] && echo "dist artifact: $f"
    done
fi

echo "released:${published} (tags:${tags})"
