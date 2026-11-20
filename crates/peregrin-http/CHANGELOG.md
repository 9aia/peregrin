# Changelog

## [1.0.0] - 2026-11-20

### Added

- **http:** kind-plane registry — /api/entities* serves registered kinds, not just sepia-session (PL-004) (`e58c782`)
- **http:** register sepia-peregrin's SepiaEntityPlane for sepia-session (PL-005) (`e38b769`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- campaign runs_on fleet labels — node caps, serve gate, entity-plane check (`ccf9c40`)
- **http,sync:** node-side /api/entities — substrate speaks entities end-to-end (`bd2b97e`)
- **BREAKING** **session:** protocol 2 + tamias-driver-sepia serve resolution (P-006 phase 3) (`a955625`)
- **sessions:** mode param on create/run + sessions.set_mode plane op (`9cff6c2`)
- **push:** cuora-backed PushBackend behind PEREGRIN_PUSH_BACKEND (`987993d`)
- **node:** ephemeral mode for CI runners — SIGTERM-aware shutdown + entity snapshot flush (`8e1dd57`)
- **campaign:** live flow lifecycle API — /api/flows on the shared listener (ADR-0093) (`8407116`)
- **node:** default the session plane to `sepia serve` (`71be7da`)
- **http:** POST /api/sessions/run — create + one blocking prompt turn (`7129803`)
- **http:** authenticated POST /api/events/inject into running campaigns (`6618c28`)
- **node:** ci-job plane rehydrates from hub entity index at boot (T-072) (`466d676`)
- **campaign:** campaign runs as entities — kind="campaign" entity plane + timer snapshot seam (P-001) (`281b769`)
- **secrets:** persistent cuora serve transport behind PEREGRIN_CUORA_TRANSPORT (`19b4ab3`)
- **http:** move projects/config/push onto hypha-kv (P-006 phase 2) (`184bec2`)
- **http:** advertise ephemeral capability on --ephemeral nodes (`011bdae`)

### Changed

- v2 — the orchestration platform, in Rust (`d6bf635`)
- **secrets:** delegate cuora impl to hypha-secret-cuora (`d523ded`)
- repoint leaf crates to hypha git deps (PL-005 P4a) (`c9ec719`)
- compose peregrin node/hub over hypha substrate (PL-005 P4b) (`bae214c`)
- **sync,hub:** kind-policy registry replaces hardcoded sepia rules (`3f6b8ff`)
- **http:** EntityPlane takes a substrate EntityContext, not AppState (`3b5c948`)

### Fixed

- rebase default ports — node :18787, hub :13000, reload :13001, dev :13002 (`b08269f`)
- **lints:** allow unknown_lints alongside clippy::assert_is_empty (`7332f3a`)
- **http:** route meta.* ops to the session plane — extra_op_namespaces (`58758f6`)
