# Changelog

## [1.0.0] - 2026-11-20

### Added

- **cli:** lib-ify the peregrin CLI and add the pavo ops surface (P-021/T-010) (`f94c570`)
- **queue:** durable supersede queue — QueueStore port, sqlite impl, supervisor replay (PL-007 C1a) (`e1247ae`)
- release infrastructure — v0.1.0 publish metadata, tag-triggered release.yml (`e098b94`)
- **queue:** wire supersede: into the campaign schema + campaign run supervisor mode (I-019) (`ce999f7`)
- campaign runs_on fleet labels — node caps, serve gate, entity-plane check (`ccf9c40`)
- peregrin distribution — source port + bundle store/cache + install verbs (`bbd3d2f`)
- **secrets:** cuora leg in the secret chains via peregrin-secret-cuora (`b1ef2a2`)
- ops surface replaces peregrin-mcp (T-021) (`ade28b3`)
- **t8-7:** campaign host — deploy drops per-bundle configs into ~/.config/peregrin/serve; bundles share provider.name=peregrin → one peregrin-campaign.service (`aae62ff`)
- **p9b:** peregrin provider — `peregrin provision install|deploy|uninstall` (tamias shells it) (`aa8fedd`)
- libraries/ci-gate+release bundles + peregrin import bupha (`a9b9c79`)
- **t8-5:** provider ops verbs — hooks register/unregister, health, status, logs (PL-008) (`a7330e6`)
- **webhook:** verified pull_request deliveries + hook opt-in (PL-007 C1d) (`a570c8c`)
- **libraries:** docs-deploy bundle — docs-branch push → gh-pages deploy campaign (PL-009 D1-D4) (`96ca067`)
- **node:** ephemeral mode for CI runners — SIGTERM-aware shutdown + entity snapshot flush (`8e1dd57`)
- **campaign:** live flow lifecycle API — /api/flows on the shared listener (ADR-0093) (`8407116`)
- **cli:** add the peregrin campaign run/validate verbs (`76552a5`)
- **t8x:** tamias-driver-peregrin config-schema — JSON Schema for the generated node config (`4f3d3e9`)
- ops verbs — create --template, deploy/destroy webhook+service+worktrees, runs/explain/doctor (`4975e82`)
- **campaign:** campaign serve — N campaigns, one shared webhook demux (PL-007 C1b) (`48a91d0`)
- **p10b:** ship tamias-driver-peregrin — the discovered content driver for peregrin-bundle (`45ffbb9`)
- **tamias:** peregrin-node bundle type — the node daemon as a deployable service (`37b3f03`)
- serve mode + mock-agent — ingress campaigns park on the channel, CLI runs ingress-aware (`21c8759`)
- **peregrin-cli:** cli verb is an engine passthrough — retire the peregrin binary (T-012) (`2150933`)
- **secrets:** persistent cuora serve transport behind PEREGRIN_CUORA_TRANSPORT (`19b4ab3`)
- **t8-4:** ci-gate tamias provider — declarative deploy via provider block (PL-008) (`059d028`)

### Changed

- **cli:** drop dead pre-tamias ops verbs; fix stale ci-gate test expectations (`fcca430`)
- v2 — the orchestration platform, in Rust (`d6bf635`)
- repoint leaf crates to hypha git deps (PL-005 P4a) (`c9ec719`)
- **p4:** hand distribution to tamias — retire peregrin-bundle/peregrin-source, tamias is the distribution (`afd4d5a`)
- **p9c:** remove create/deploy/destroy + service verbs from peregrin; tamias owns provisioning/service (`76002c9`)
- **p9c:** remove distribution verbs from peregrin — tamias owns distribution (`5fead64`)
- **p7:** drop peregrin doctor + ops doctor internals (doctor lives in tamias) — ops.rs pruned (`5bb8c2b`)
- **BREAKING** **cli:** retire embedded mcp surface — ops manifest is the agent contract (P-028/T-021) (`440ed6a`)

### Fixed

- **cli,mcp:** wiring validator accepts wildcard extensions of templated emits (`d5e9cf7`)
- ci-gate runnable end-to-end — capabilities grant, repo routing, sink fallback, serve-mode test fixes (`ceb03af`)
- rebase default ports — node :18787, hub :13000, reload :13001, dev :13002 (`b08269f`)
- **cli:** pair URL honors PEREGRIN_PORT; mark token params secret + bump pavo to a7aff58 (`945562b`)
- **lints:** allow unknown_lints alongside clippy::assert_is_empty (`7332f3a`)
- **driver:** secret resolution chain — ambient env + declared keyring legs (I-018) (`6dedd06`)
- **xtask:** link peregrin bins + stage public/ assets for e2e harness (`35e87a2`)
- point cli tests at the peregrin binary (`0d316dc`)
