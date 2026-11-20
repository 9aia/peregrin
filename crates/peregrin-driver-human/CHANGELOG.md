# Changelog

## [0.3.0] - 2026-11-20

### Added

- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **driver:** port the ac-driver-human campaign driver (`6cd65b1`)
- scope-resolution pass — ${scope.path} resolution in peregrin-template (`254640a`)

### Fixed

- **lints:** allow unknown_lints alongside clippy::assert_is_empty (`7332f3a`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
