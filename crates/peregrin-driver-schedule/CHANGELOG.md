# Changelog

## [0.3.0] - 2026-11-20

### Added

- **driver:** temporal scheduling drivers — delay + schedule (ADR-0018) (`eb18a5c`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **driver-schedule,libraries:** delay passthrough + migrate retry/circuit-breaker presets off shell sleep (`277bb64`)

### Fixed

- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
