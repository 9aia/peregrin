# Changelog

## [1.0.0] - 2026-11-20

### Added

- register github-status sink in the node driver registry (`ff74224`)
- **node:** host ci-job plane + ci-ingress campaign (P-002, T-061/T-058/T-063) (`fba0fc5`)
- register git driver in the node registry (inner-driver lookup over base set) (`f2c7266`)
- **campaign:** pause/resume checkpoint ops + live run projection on the entity plane (`ee8a3b2`)
- **driver:** temporal scheduling drivers — delay + schedule (ADR-0018) (`eb18a5c`)
- **driver:** council voting driver — one node as a voting round (ADR-0004) (`e7a1bf5`)
- **queue:** durable supersede queue — QueueStore port, sqlite impl, supervisor replay (PL-007 C1a) (`e1247ae`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **queue:** wire supersede: into the campaign schema + campaign run supervisor mode (I-019) (`ce999f7`)
- campaign runs_on fleet labels — node caps, serve gate, entity-plane check (`ccf9c40`)
- **engine:** live timer mirror — mid-run campaign snapshots carry armed timers (`c87312a`)
- **node:** superseded-run observability markers (PL-007 C1c) (`b938c35`)
- **ci-ingress:** fleet normalization, pull_request lane, deliveryId, enqueue retries (T-067) (`b6dbf98`)
- **secrets:** cuora leg in the secret chains via peregrin-secret-cuora (`b1ef2a2`)
- **BREAKING** **session:** protocol 2 + tamias-driver-sepia serve resolution (P-006 phase 3) (`a955625`)
- **libraries:** map/reduce fragment — fan-out via planner+inject, fan_in reduce (ADR-0021) (`9bb078b`)
- **driver:** ops driver — bounded tamias-driver queries over tamias serve (P-096) (`9ba1cd0`)
- **node:** ephemeral mode for CI runners — SIGTERM-aware shutdown + entity snapshot flush (`8e1dd57`)
- **campaign:** live flow lifecycle API — /api/flows on the shared listener (ADR-0093) (`8407116`)
- peregrin-queue supersede policy + ingress injection channel (`7d55fb4`)
- **node:** default the session plane to `sepia serve` (`71be7da`)
- **http:** authenticated POST /api/events/inject into running campaigns (`6618c28`)
- **campaign:** campaign serve — N campaigns, one shared webhook demux (PL-007 C1b) (`48a91d0`)
- **node:** ci-job plane rehydrates from hub entity index at boot (T-072) (`466d676`)
- runner_with_ingress — run-scoped ingress drivers (github-webhook, sturn-poll) wired to Injector (`323c039`)
- **node:** compose the in-process campaign plane (`2b152f0`)
- libraries/{retry,circuit-breaker,dead-letter} resilience fragments (`28b4e52`)
- **campaign:** campaign runs as entities — kind="campaign" entity plane + timer snapshot seam (P-001) (`281b769`)
- **driver-schedule,libraries:** delay passthrough + migrate retry/circuit-breaker presets off shell sleep (`277bb64`)
- **driver:** subworkflow driver — nested campaign runs (ADR-0022) (`25f493d`)
- serve mode + mock-agent — ingress campaigns park on the channel, CLI runs ingress-aware (`21c8759`)
- campaign::run wires ingress drivers + populates RunOptions.secrets from env file (`1ca4a26`)
- **secrets:** persistent cuora serve transport behind PEREGRIN_CUORA_TRANSPORT (`19b4ab3`)
- peregrin-store-json — durable JSONL event store (`1513874`)

### Changed

- **cli:** drop dead pre-tamias ops verbs; fix stale ci-gate test expectations (`fcca430`)
- v2 — the orchestration platform, in Rust (`d6bf635`)
- repoint leaf crates to hypha git deps (PL-005 P4a) (`c9ec719`)
- compose peregrin node/hub over hypha substrate (PL-005 P4b) (`bae214c`)
- **http:** EntityPlane takes a substrate EntityContext, not AppState (`3b5c948`)

### Fixed

- **secrets:** batch secret_env resolution over parallel cuora serve lanes (T-076) (`ff46599`)
- ci-gate runnable end-to-end — capabilities grant, repo routing, sink fallback, serve-mode test fixes (`ceb03af`)
- **test:** place driver binaries cross-device safely (`a4d7bc4`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
- **supervisor:** listener run gets the shared history sink (T-076) (`0829a43`)
