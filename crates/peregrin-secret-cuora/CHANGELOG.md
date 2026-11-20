# Changelog

## [0.3.0] - 2026-11-20

### Added

- **secret-cuora:** sealed-pack fallback leg for secrets-ci-webhooks (T-053) (`ff1a8a1`)
- **secrets:** cuora leg in the secret chains via peregrin-secret-cuora (`b1ef2a2`)
- **secrets:** persistent cuora serve transport behind PEREGRIN_CUORA_TRANSPORT (`19b4ab3`)

### Changed

- **secrets:** delegate cuora impl to hypha-secret-cuora (`d523ded`)

### Fixed

- **secrets:** batch secret_env resolution over parallel cuora serve lanes (T-076) (`ff46599`)
