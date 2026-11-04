+++
id = "ADR-0080"
title = "peregrin as orchestration substrate — trigger/driver/sink planes, absorb CI gate"
status = "open"
created = "2026-11-04"
priority = "medium"
+++

# peregrin as orchestration substrate — trigger/driver/sink planes, absorb CI gate

Status: **proposed** · Created: 2026-11-04

## Context

The 9aia factory is converging on one event substrate: sturn owns
ephemeral coordination (WAL in `refs/sturn/*`, claims, topics), myco
owns durable state, sepia owns session mechanism — and peregrin owns
policy + orchestration on top. Meanwhile bupha re-implements the same
vertical in miniature: a GitHub-webhook trigger listener, a queue
(latest-wins batching), a runner (detached worktree at the pushed
sha), and a status sink (`ci/bupha`). A "push → run → verdict" gate
is a degenerate peregrin campaign — one trigger source, one driver,
one sink.

Two webhook listeners, two queue models, two systemd daemons is
duplicated machinery. If peregrin is the substrate, the CI gate
belongs in it as a *distribution*, not a sibling binary.

## Decision

- **Peregrin is the substrate for orchestration/execution.** Event
  plane in (triggers), execution out (drivers, sinks), campaigns as
  policy between them. The trigger/driver split already exists;
  **sinks become a first-class plane** — effect endpoints that
  publish verdicts out (github commit-status, WAL writes, webhooks)
  rather than ad-hoc side effects inside runners.
- **Grow the port families, don't special-case CI.** Missing impls
  land as generic ports so other campaigns reuse them:
  `trigger-github` (HMAC listener + payload → trigger event),
  `trigger-sturn` (WAL ref poller — `sturn snap` → run for free),
  `driver-git` (checkout sha → detached worktree → invoke inner
  driver), `sink-github-status` (commit status posts).
- **bupha becomes a bundle** (`libraries/ci-gate`, via ADR-0079
  install) — `peregrin create --template ci-gate` = install +
  enable. (`create --template`, not `init <profile>` — profiles
  aren't hardcoded in the CLI; the template resolves against the
  store so any bundle works.)
- **Config is engine vocabulary, not CI vocabulary.** bupha's
  `[[repos]]` conflated two things — which events a source watches
  and what a campaign does with them. They split: `triggers:` holds
  the watch list (`kind: github`, `repo:`, `secret_env:` — source
  config), `campaigns:` hold `on:` filters routing events to
  params (repo/ref/sha arrive *in the event*, driver-git checks out
  from `clone_url` — no per-repo `dir`). bupha.toml/yaml maps onto
  this shape rather than surviving verbatim.
- **Migration**: build impls → shadow-run posting `ci/peregrin`
  beside `ci/bupha` → compare verdicts → cutover hooks → retire the
  watcher unit + `bupha.toml`→campaign importer for existing configs.
- **Node profiles**: a gate node runs a minimal slice (triggers +
  git driver + status sink — no sessions, no web). `peregrin create
  --template <bundle>` materializes the unit + config so the gate keeps an
  "always-up" personality inside the bigger engine.
- **Service scoping rule** — execution semantics native; service
  surfaces stay external or subprocess: sepia (session mechanism,
  done), arca (secrets vault — configs call it, never embedded),
  cloudflared tunnels (subprocess, not owned). Telemetry *emit* is
  native (instrumenting runs is execution data); the OTLP
  collector/dashboard stack is external infra. `peregrin-web`/hub
  stays for now but is human UI only — no policy.
- **Agent surface** — the substrate must be drivable by agents:
  `.agents/skills/` describing the verbs + an MCP server exposing
  them (sturn-mcp is the model). Not a side feature — it's the
  primary control surface alongside the human CLI/UI.

## Consequences

- One queue, one event model, one daemon — but the gate's
  availability now shares the engine's blast radius; the
  minimal-profile gate node is the mitigation, and bupha stays
  as a *frozen* binary for boxes where a full node is overkill
  (ADR-0081).
- Sinks as a plane lets non-CI campaigns publish verdicts the same
  way (review verdicts back to sturn, notifications, deploy calls).
- `trigger-sturn` makes agent-triggered CI native: `snap` → run —
  the factory's coordination plane feeds its execution plane
  without a webhook round-trip.
- ADR-0079 (install/remove + store) is the distribution half of
  this decision — bundles like `ci-gate` ship through it.
