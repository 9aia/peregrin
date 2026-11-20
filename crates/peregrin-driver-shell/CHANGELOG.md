# Changelog

## [0.3.0] - 2026-11-20

### Added

- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- driver-shell isolate/memory_max via systemd-run user scope (`d83d4dd`)
- **engine:** port the ac-engine runner, bus, ports, and safety (`48b9c8e`)
- scope-resolution pass — ${scope.path} resolution in peregrin-template (`254640a`)
- **loader:** port the ac-loader-yaml campaign loader (`1b3bf42`)
- peregrin-driver-git — detached worktree-at-sha driver (`1064bf8`)

### Changed

- v2 — the orchestration platform, in Rust (`d6bf635`)

### Fixed

- scrub spawned-child env, bound pipe capture, honor shell timeout (`841c9ac`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
