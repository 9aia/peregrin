# Changelog

## [0.3.0] - 2026-11-20

### Added

- **template:** port the ac-template-engine to peregrin-template (`ffe5d87`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **engine:** port the ac-engine runner, bus, ports, and safety (`48b9c8e`)
- scope-resolution pass — ${scope.path} resolution in peregrin-template (`254640a`)
- **loader:** port the ac-loader-yaml campaign loader (`1b3bf42`)

### Changed

- v2 — the orchestration platform, in Rust (`d6bf635`)

### Fixed

- scrub spawned-child env, bound pipe capture, honor shell timeout (`841c9ac`)
- **lints:** allow unknown_lints alongside clippy::assert_is_empty (`7332f3a`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
