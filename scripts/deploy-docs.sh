#!/usr/bin/env bash
# Build the VitePress site and publish .vitepress/dist to the `gh-pages`
# branch. Runs identically by hand (from the docs/ worktree) and under
# bupha — the watcher runs `bash scripts/deploy-docs.sh` in a detached
# worktree of the pushed docs sha. This site is hand-maintained — no
# codegen step.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# The bupha watcher unit runs with the minimal systemd PATH
# (/usr/local/bin:/usr/bin) whenever it starts before the session env is
# imported — pull the user tool dirs in ourselves so bun resolves like
# it does in a login shell.
export PATH="$HOME/.bun/bin:$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

echo "==> bun install"
bun install

echo "==> vitepress build"
bunx vitepress build

# Branch deploys still get a Jekyll pass unless .nojekyll is present.
touch .vitepress/dist/.nojekyll

echo "==> deploy .vitepress/dist → gh-pages"
bunx gh-pages -d .vitepress/dist -b gh-pages \
  -r git@github.com:9aia/peregrin.git \
  -m "docs: deploy $(git rev-parse --short HEAD)"

echo "done — https://9aia.com/peregrin/ serves origin/gh-pages"
