# Changelog

## [0.3.0] - 2026-11-20

### Added

- **queue:** durable supersede queue — QueueStore port, sqlite impl, supervisor replay (PL-007 C1a) (`e1247ae`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **queue:** wire supersede: into the campaign schema + campaign run supervisor mode (I-019) (`ce999f7`)
- **campaign:** live flow lifecycle API — /api/flows on the shared listener (ADR-0093) (`8407116`)
- peregrin-queue supersede policy + ingress injection channel (`7d55fb4`)
