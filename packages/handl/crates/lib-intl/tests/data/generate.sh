#!/bin/sh
# Generate test.postcard blob for blob_integration tests.
#
# Requires icu4x-datagen:
#   cargo install icu4x-datagen
#
# Usage:
#   sh packages/handl/crates/lib-intl/tests/data/generate.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT="$SCRIPT_DIR/test.postcard"

icu4x-datagen \
  --keys core/decimal/symbols@1 core/plurals@1 core/list/and@1 core/list/or@1 core/list/unit@1 \
  --locales en,de,ja,es,pt-BR,ar,bn,sv,fr \
  --format blob \
  --out "$OUT"

echo "Generated $OUT"
