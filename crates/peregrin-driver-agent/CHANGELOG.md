# Changelog

## [0.3.0] - 2026-11-20

### Added

- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **driver:** port the ac-driver-agent campaign driver (`9bdbdf3`)
- scope-resolution pass — ${scope.path} resolution in peregrin-template (`254640a`)
- serve mode + mock-agent — ingress campaigns park on the channel, CLI runs ingress-aware (`21c8759`)

### Fixed

- scrub spawned-child env, bound pipe capture, honor shell timeout (`841c9ac`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
