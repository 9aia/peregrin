# Changelog

## [0.3.0] - 2026-11-20

### Added

- **driver:** temporal scheduling drivers — delay + schedule (ADR-0018) (`eb18a5c`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **queue:** wire supersede: into the campaign schema + campaign run supervisor mode (I-019) (`ce999f7`)
- **hub:** fleet dispatch — route runs_on campaign creates to eligible nodes (PL-007 C4) (`ce05fe8`)
- campaign runs_on fleet labels — node caps, serve gate, entity-plane check (`ccf9c40`)
- **engine:** port the ac-engine runner, bus, ports, and safety (`48b9c8e`)
- **driver:** subworkflow driver — nested campaign runs (ADR-0022) (`25f493d`)
- scope-resolution pass — ${scope.path} resolution in peregrin-template (`254640a`)
- **loader:** port the ac-loader-yaml campaign loader (`1b3bf42`)

### Changed

- v2 — the orchestration platform, in Rust (`d6bf635`)

### Fixed

- **cli,mcp:** wiring validator accepts wildcard extensions of templated emits (`d5e9cf7`)
- scrub spawned-child env, bound pipe capture, honor shell timeout (`841c9ac`)
- **lints:** allow unknown_lints alongside clippy::assert_is_empty (`7332f3a`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
