# Changelog

## [0.3.0] - 2026-11-20

### Added

- **node:** host ci-job plane + ci-ingress campaign (P-002, T-061/T-058/T-063) (`fba0fc5`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **hub:** arbitrate session writes through the lock registry (`da515c9`)
- **hub:** fleet dispatch — route runs_on campaign creates to eligible nodes (PL-007 C4) (`ce05fe8`)
- **http,sync:** node-side /api/entities — substrate speaks entities end-to-end (`bd2b97e`)
- **hub:** map PEREGRIN_EPHEMERAL_DEAD_AFTER → HYPHA_EPHEMERAL_DEAD_AFTER (`9ad16bf`)
- **hub:** merged /api/entities HTTP surface over the projection + outbox (`8d84368`)
- **hub:** gate campaign.* ops on the entity lock via CampaignKindPolicy (`17ceac7`)

### Changed

- v2 — the orchestration platform, in Rust (`d6bf635`)
- repoint leaf crates to hypha git deps (PL-005 P4a) (`c9ec719`)
- compose peregrin node/hub over hypha substrate (PL-005 P4b) (`bae214c`)
- **sync,hub,node-api:** entity plane — de-chat-shape the distributed layer (ADR-0091) (`a3525a3`)
- **web:** move Leptos UI off main — headless hub on the stable line (`917b382`)
- **hub,web:** extract peregrin-node-api — the leptos-free node port (`5da66b4`)
- **sync,hub:** kind-policy registry replaces hardcoded sepia rules (`3f6b8ff`)

### Fixed

- harden secret-file writes, store perms, and hub SSE url (`ee40b83`)
- rebase default ports — node :18787, hub :13000, reload :13001, dev :13002 (`b08269f`)
- **test:** place driver binaries cross-device safely (`a4d7bc4`)
- **sync:** SSE streams must outlive ureq's total body deadline (`9c8f30d`)
