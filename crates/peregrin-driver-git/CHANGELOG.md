# Changelog

## [0.3.0] - 2026-11-20

### Added

- **driver:** council voting driver — one node as a voting round (ADR-0004) (`e7a1bf5`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **driver-git,docs-deploy:** extra_worktrees dual-checkout + gen-dependent repos (PL-009 D6) (`c55e4fd`)
- peregrin-driver-git — detached worktree-at-sha driver (`1064bf8`)
- **t8-4:** ci-gate tamias provider — declarative deploy via provider block (PL-008) (`059d028`)

### Fixed

- **driver-git:** fetch cache with explicit refspec — bare clone writes none (`ee046d1`)
- ci-gate runnable end-to-end — capabilities grant, repo routing, sink fallback, serve-mode test fixes (`ceb03af`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
- **driver-git:** Basic auth for token_env — bearer rejected for gho_ OAuth tokens (`609f1e4`)
