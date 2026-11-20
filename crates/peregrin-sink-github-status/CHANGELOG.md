# Changelog

## [0.3.0] - 2026-11-20

### Added

- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- peregrin-sink-github-status — commit-status egress driver (ci/<campaign>) (`c4070e6`)

### Fixed

- ci-gate runnable end-to-end — capabilities grant, repo routing, sink fallback, serve-mode test fixes (`ceb03af`)
- **lints:** allow unknown_lints alongside clippy::assert_is_empty (`7332f3a`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
