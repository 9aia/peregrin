+++
id = "ADR-0081"
title = "bupha→peregrin migration inventory — queue semantics, drivers, sinks, ops"
status = "open"
created = "2026-11-04"
priority = "medium"
+++

# bupha→peregrin migration inventory — queue semantics, drivers, sinks, ops

Status: **proposed** · Created: 2026-11-04

## Context

ADR-0080 decides that peregrin absorbs the CI gate as a `ci-gate`
bundle. This ADR is the inventory of what must exist in peregrin
for that to work — bupha's full surface mapped onto peregrin's
planes, so "CI is just a config" is true rather than aspirational.

## Decision

Capabilities peregrin needs, by plane:

- **Queue (the real migration)** — semantics live in the engine:
  latest-wins supersede keyed on `(repo, ref)`, sequential mode,
  `cancel_running` — all **per-campaign configurable** (locked in
  review). Needs a *supersede key* concept in the queue policy.
- **Triggers (new)** — `trigger-github` (HMAC listener, per-repo
  secrets, cloudflared tunnel hand-off) and `trigger-sturn` (WAL
  ref poller — `snap → run` natively).
- **Drivers** — `driver-git` (detached worktree at pushed sha),
  shell (exists), `driver-dagger` (or script-wrap), remote runs
  via `remote-http` (exists), and cgroup isolation as a driver
  option (`isolate: scope`, `memory_max` — the OOM-containment
  story from bupha's graphopt loop).
- **Sinks (new plane per ADR-0080)** — `sink-github-status` with
  `status_repo` redirect and deterministic `ci/<campaign>` contexts
  (locked in review): shadow posts `ci/<campaign>` beside legacy
  `ci/bupha`; cutover migrates protection rules once, then
  `ci/bupha` retires.
- **Secrets** — parity already (env → keyring → env-file).
- **Ops verbs** — webhook registration + systemd unit deploy/
  toggle → `peregrin create --template` (template bundles
  materialize the unit + config) + `deploy` verbs.
- **Observability** — `history.jsonl`/`runs`/`explain`/`doctor` →
  run records in peregrin-store + CLI verbs (events feed exists).
- **Fleet** — `runs_on` labels → node labels; `worker`/`agent`
  remote execution → existing remote drivers.
- **Artifacts** — `libraries/ci-gate` + a `release` bundle;
  `bupha.toml`/`bupha.yaml` → campaign-param importer for the
  ~70-entry live config.

Locked in review (2026-11-04): `on:` filters are **glob** matching
(repo/ref patterns, no expression language); supersede policy is
**per-campaign configurable** (key, mode, cancel); **secrets bind
per-impl in node config** — campaigns get declared capabilities,
never env-var names (a bundle can't widen a secret's blast
radius); run worktrees have an **explicit lifecycle** (created →
kept-for-debug → destroyed, librarizable helper); status contexts
are **`ci/<campaign>`**, deterministic; bundle params validate
against **JSON Schema** (field.yml, ADR-0032). Open: the **event
envelope** schema — the contract every impl/campaign speaks —
drafts next before impls.

Cut: **`peregrin` is general — the gate is configuration.** The
config speaks engine vocabulary: `triggers:` (watch lists per
source), `campaigns:` (`on:` event filters + driver/sink params).
bupha's `repos:`/`jobs:` map onto it — repo+secret → trigger entry,
job branch → `on: ref` filter, runner cmd → driver params — and no
CI-specific engine code lands beyond the generic ports above.

## Consequences

- Ordered build: queue-supersede semantics → driver-git →
  github trigger/status pair → bundle + importer → shadow run →
  cutover. Most other items are config-shape ports.
- bupha stays runnable for minimal boxes — **frozen binary, shared
  bundle refs.** Drift is neutralized by integrity: digest-pinned
  bundles mean remote moves are detected (`verify`/`update`), not
  silent. `#sha` pins are immutable; `#tag`/`#branch` move on
  explicit install. The substrate absorbs the orchestrated case,
  not the standalone one.
