# Changelog

## [0.3.0] - 2026-11-20

### Added

- **driver:** temporal scheduling drivers — delay + schedule (ADR-0018) (`eb18a5c`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **engine:** live timer mirror — mid-run campaign snapshots carry armed timers (`c87312a`)
- peregrin-queue supersede policy + ingress injection channel (`7d55fb4`)
- **engine:** port the ac-engine runner, bus, ports, and safety (`48b9c8e`)
- **campaign:** campaign runs as entities — kind="campaign" entity plane + timer snapshot seam (P-001) (`281b769`)
- **driver:** subworkflow driver — nested campaign runs (ADR-0022) (`25f493d`)
- scope-resolution pass — ${scope.path} resolution in peregrin-template (`254640a`)
- serve mode + mock-agent — ingress campaigns park on the channel, CLI runs ingress-aware (`21c8759`)

### Changed

- v2 — the orchestration platform, in Rust (`d6bf635`)
- repoint leaf crates to hypha git deps (PL-005 P4a) (`c9ec719`)

### Fixed

- **secrets:** batch secret_env resolution over parallel cuora serve lanes (T-076) (`ff46599`)
- ci-gate runnable end-to-end — capabilities grant, repo routing, sink fallback, serve-mode test fixes (`ceb03af`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
