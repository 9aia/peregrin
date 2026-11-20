# Changelog

## [0.3.0] - 2026-11-20

### Added

- **webhook:** push_refs param — per-route push ref filtering (`ee8f442`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **ci-ingress:** fleet normalization, pull_request lane, deliveryId, enqueue retries (T-067) (`b6dbf98`)
- **webhook:** verified pull_request deliveries + hook opt-in (PL-007 C1d) (`a570c8c`)
- **ci-ingress:** fork-PR head sha support — cloneUrl/headRef on ci-job (T-073) (`a1cd2be`)
- **driver-github-webhook:** check_run/check_suite/status deliveries (`88176ad`)
- **campaign:** live flow lifecycle API — /api/flows on the shared listener (ADR-0093) (`8407116`)
- **campaign:** campaign serve — N campaigns, one shared webhook demux (PL-007 C1b) (`48a91d0`)
- peregrin-driver-github-webhook — HMAC-verified push ingress (`273f44c`)
- serve mode + mock-agent — ingress campaigns park on the channel, CLI runs ingress-aware (`21c8759`)

### Fixed

- **secrets:** batch secret_env resolution over parallel cuora serve lanes (T-076) (`ff46599`)
- ci-gate runnable end-to-end — capabilities grant, repo routing, sink fallback, serve-mode test fixes (`ceb03af`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
