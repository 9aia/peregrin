+++
id = "ADR-0091"
title = "Generic entity plane — distributed primitives over workloads, not sessions"
status = "open"
created = "2026-11-05"
owner = "devin"
priority = "high"
+++

# Generic entity plane — distributed primitives over workloads, not sessions

## Status

Proposed

## Context

Peregrin is a generic multi-machine orchestrator: campaigns, nodes, drivers, ports. The session-management app made its *distributed plane* chat-shaped — the code names AI-session concepts all the way down:

- `peregrin-node-api` — `SessionSummaryDto`, `prompt`/`attach`/`history`/`rewind` verbs, transcript IR transfer
- `peregrin-sync` — projects each node's *session index* into a local SQLite projection
- `peregrin-outbox` — per-session FIFO of chat writes (prompt, meta patches)
- `peregrin-control` — arbitration over *session* locks (takeover, merge/route)
- `peregrin-push` — notifications on session lifecycle events
- `peregrin-hub` — the session gateway (merged view + op routing)

Meanwhile sepia is deliberately *just* the AI-agent abstraction: it generalizes agents (cline, devin, claude, cursor), skills, tools, MCP — a service, nothing more. It must never learn about locks, multi-node, or orchestration.

Two conclusions follow. (a) The distributed abilities — sync, control, outbox, transfer/resume, pairing — are **orchestration-domain** and belong in peregrin. (b) Their current *session* shape is wrong: peregrin must be totally generic, able to serialize state and resume it on other machines for **any** workload. Orchestrating AI sessions is one possible thing; sepia becomes a first-class *integration*, not a peer the substrate is shaped around.

## Decision

Introduce the **entity plane**: peregrin's distributed primitives operate on generic `Entity` values, never session types.

### The entity model

- `Entity` — `{id, kind, node, indexDoc, health}`. `kind` is a free-form tag (`"sepia-session"`, `"campaign"`, …); `indexDoc` is an opaque JSON blob the integration writes for list views; `health` is liveness/lock state.
- `Op` — `{entity, kind, payload}` submitted to an entity's owner node; payloads are opaque to peregrin (`"prompt"`, `"attach"`, `"custom.*"` are integration-defined op kinds).
- `Snapshot` — `{entity, ref, bytes}` — serialize → transfer → resume on another node. Export/import is kind-specific and lives in the integration, not the substrate.

### Crate changes (de-chat-shaping)

| crate | becomes |
| --- | --- |
| `peregrin-node-api` | generic node↔entity contract: `list_entities`, `entity`, `ops`, `snapshot export/import`, index tail. `SessionSummaryDto` & friends leave the crate |
| `peregrin-sync` | entity-index projection (kind-agnostic docs + a `kind` column), node health |
| `peregrin-outbox` | per-entity op queue; op kinds are `kind`-namespaced, not chat verbs |
| `peregrin-control` | entity lock registry + takeover/merge/route — "who may drive entity X on node Y" |
| `peregrin-push` | generic event→notification map; session lifecycle rules move to the integration |
| `peregrin-hub` | entity gateway: merged index + op routing + SSE/passthrough (already decoupled post-web-split) |
| `peregrin-auth` | unchanged — pairing/credentials are already generic |

### The sepia integration

A `sepia-peregrin` adapter crate **lives in the sepia repo** (path/git dep direction: sepia-side → peregrin public API, never the reverse). It:

- registers `kind="sepia-session"` entities — a sepia session *is* an entity;
- implements snapshot as the session IR export/import it already owns;
- translates entity ops ↔ session verbs (`prompt`, `attach`, `rewind`, `permission`…);
- supplies the session-shaped DTO surface (`SessionSummary`, history, capabilities) the web UI consumes — those types leave peregrin entirely.

Rule: **no peregrin crate may name a sepia/session type.** Violations are found by grep, not convention.

## Consequences

- The session-app plane stays in peregrin but becomes reusable: campaigns become entities too (multi-node campaign state rides the same sync/outbox/transfer machinery).
- The `experimental/web` branch is unaffected short-term: its crates keep the old session API on that branch; the session UI migrates onto `sepia-peregrin` when it stabilizes.
- Wire/schema breaks are expected (projection schema, NodeApi surface) — the session app is experimental; no compat shim is promised beyond the transition plan.
- Risk: this is a substrate rewrite touching sync, outbox, control, hub + their test suites. Phased plan: `plans/pl-0004` (tracked via myco).
