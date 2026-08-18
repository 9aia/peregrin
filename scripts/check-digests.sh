#!/usr/bin/env bash
# Check for stale digests — source rule files modified after their digests.
# Exit 0 if all digests are current, exit 1 if any are stale.

set -euo pipefail

RULES_DIR="$(cd "$(dirname "$0")/.." && pwd)/rules"
DIGEST_DIR="$RULES_DIR/.digest"
STALE=0

for rule_file in "$RULES_DIR"/*.md; do
    name=$(basename "$rule_file")

    # Skip files that don't have digests by design
    case "$name" in
        README.md|principles.md|git-conventions.md|ci-cd.md|testing.md|workspace.md) continue ;;
    esac

    digest_file="$DIGEST_DIR/$name"

    if [[ ! -f "$digest_file" ]]; then
        echo "MISSING digest: $name (no corresponding digest in .digest/)"
        STALE=1
        continue
    fi

    if [[ "$rule_file" -nt "$digest_file" ]]; then
        echo "STALE digest: $name (source modified after digest)"
        STALE=1
    fi
done

if [[ $STALE -eq 0 ]]; then
    echo "All digests are current."
fi

exit $STALE
