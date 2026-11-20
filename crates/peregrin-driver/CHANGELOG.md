# Changelog

## [0.3.0] - 2026-11-20

### Added

- **driver:** temporal scheduling drivers — delay + schedule (ADR-0018) (`eb18a5c`)
- **driver:** council voting driver — one node as a voting round (ADR-0004) (`e7a1bf5`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **libraries:** map/reduce fragment — fan-out via planner+inject, fan_in reduce (ADR-0021) (`9bb078b`)
- **engine:** port the ac-engine runner, bus, ports, and safety (`48b9c8e`)
- **driver:** subworkflow driver — nested campaign runs (ADR-0022) (`25f493d`)
- scope-resolution pass — ${scope.path} resolution in peregrin-template (`254640a`)
- serve mode + mock-agent — ingress campaigns park on the channel, CLI runs ingress-aware (`21c8759`)
- **loader:** port the ac-loader-yaml campaign loader (`1b3bf42`)

### Changed

- v2 — the orchestration platform, in Rust (`d6bf635`)
- repoint leaf crates to hypha git deps (PL-005 P4a) (`c9ec719`)

### Fixed

- **secrets:** batch secret_env resolution over parallel cuora serve lanes (T-076) (`ff46599`)
- **lints:** allow unknown_lints alongside clippy::assert_is_empty (`7332f3a`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
