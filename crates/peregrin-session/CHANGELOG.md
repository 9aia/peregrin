# Changelog

## [1.0.0] - 2026-11-20

### Added

- **http:** register sepia-peregrin's SepiaEntityPlane for sepia-session (PL-005) (`e38b769`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **BREAKING** **session:** protocol 2 + tamias-driver-sepia serve resolution (P-006 phase 3) (`a955625`)
- **sessions:** mode param on create/run + sessions.set_mode plane op (`9cff6c2`)
- **node:** default the session plane to `sepia serve` (`71be7da`)
- **http:** POST /api/sessions/run — create + one blocking prompt turn (`7129803`)

### Fixed

- **session:** budget prompt_wait wire call for the full turn — transport default killed real agent turns at ~30s (`c814044`)
