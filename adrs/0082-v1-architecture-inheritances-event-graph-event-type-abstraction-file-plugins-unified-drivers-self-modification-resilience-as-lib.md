+++
id = "ADR-0082"
title = "v1 architecture inheritances — event graph, event-type abstraction, file plugins, unified drivers, self-modification, resilience-as-lib"
status = "open"
created = "2026-11-04"
priority = "medium"
+++

# v1 architecture inheritances — event graph, event-type abstraction, file plugins, unified drivers, self-modification, resilience-as-lib

Status: **proposed** · Created: 2026-11-04

## Context

Peregrin v1 (the TypeScript implementation, tree preserved at
`d6bf6356~1` — `docs/ac/explanation/` + `packages/ac-*` +
`libraries/`) contained a complete, battle-tested architecture
writeup that the v2 Rust rewrite and ADRs 0079–0081 had partially
re-derived and partially forgotten. Reviewed 2026-11-04; the seven
load-bearing decisions below are confirmed as inherited — they were
"EXTREMELY important and well thought out" in v1 and remain the
right answers.

## Decision

1. **Event graph, not DAG** — nodes subscribe to event types;
   branching, fan-out, fan-in (`mode: all`), dynamic topology, and
   wildcards (`shard.*.ready`) are all free consequences of
   subscription. `on:` filters (ADR-0081's glob decision) subscribe
   to event types — wildcards included.
   (v1: `docs/ac/explanation/event-graph-model.md`)
2. **Abstraction via event types** — nodes emit *abstract intent*
   (`request-notify`); subscribers implement (desktop, log).
   Swapping an implementation = swapping one subscriber, zero
   changes upstream. Sinks and multi-impl dispatch are instances of
   this — subscribe to intent, not to sources.
   (v1: `abstraction.md`)
3. **"No plugin system: files"** — install = drop a YAML file,
   version = git, compose = `include:`, **override = deep-merge**,
   remove = delete the include. ADR-0079's bundle distribution is
   the transport half of exactly this; **deep-merge override
   composition** (bundles extending/overlaying each other and local
   values) is the missing half — bundles compose by merge, not by
   registry.
   (v1: `plugins.md`, `include-resolution.md`)
4. **Unified driver registry — triggers are drivers.** v1 had no
   trigger plane: `webhook` (HTTP server), `cli` (poll), `stdin`,
   `agent`, `human` were drivers that emit events. v2 keeps that —
   one registry; "triggers" (ADR-0080) are *ingress drivers*
   (long-running drivers that emit), sinks are *egress drivers*
   (drivers that consume terminal events). Preferred over the
   separate-plane split: fewer concepts, same extension point.
   (v1: `drivers.md`)
5. **Self-modifying workflows** — a `runner` driver injects nodes
   into the live graph from event payloads (`retire_on` scoping);
   **loop protection in three layers** (`max_fires`, `until`,
   `retire_on`). Dynamic topology is core, not a feature flag.
   (v1: `self-modifying.md`, `loop-protection.md`)
6. **Resilience as library, not engine** — circuit-breaker,
   dead-letter queue, retry loops are composable workflow artifacts
   (libraries/ entries), not engine hardcode — the
   "hardcode → distributable libs" rule again, same spirit as
   ADR-0081's config-into-campaign mapping.
   (v1: `resilience.md`, `reference/circuit-breaker.md`,
   `dead-letter-queue.md`)
7. **Observers as an adapter port** — `NoopObserver` default,
   `OTelObserver` — telemetry emit is a swappable port, which is
   precisely ADR-0080's "emit native, stack external" rule.
   (v1: `observers.md`, `adapter-ports.md`)

## Consequences

- ADR-0080's "trigger plane" language is superseded by the unified
  registry: `trigger-github`/`trigger-sturn` become ingress drivers;
  `sink-github-status` an egress driver. Plane *names* may stay as
  docs vocabulary — the registry is one.
- The v1 explanation docs remain the engine spec source-of-truth
  for the concepts above; v2 impls should cite them rather than
  re-invent semantics.
- Threat-model/safety docs from v1 (`threat-model.md`,
  `safety.md`) remain worth porting — AI-governed workflow safety
  is still a first-class concern.
- Not inherited blindly: v1's stores (`JsonStore`/`MemoryStore`)
  map onto the existing `peregrin-store` port; v2 keeps its own
  adapter set.

## Audit inventory (2026-11-04)

Deep audit of the v1 tree (`d6bf6356~1`: `docs/ac/` +
`packages/ac-*` + `libraries/`) against the v2 port — what already
exists, what PL-003 still has to build, and what needs a ruling or
fix.

### Exists (ported, verified)

- `mode: all` fan-in — first match per distinct `on` pattern
- Wildcards + captures — `*`/`**`, `match.N`, `shard` alias
- `runner` injection — `ctx.inject` mid-run node specs
- 3-layer loop protection — `max_fires` / `until` / `retire_on`
- Deep-merge `include` — dict-level key + `${include(path)}`
- Observer trait — `NoopObserver` default, `Span` hooks
- `emits`/`emits` contract — declaration, strict exact check,
  `params.emit`, `ctx.emit`, `forward`
- Driver input/output schemas — `fields`/`schema` compile,
  node-over-driver override
- `fan_in` / `forward` node mechanics
- Custom functions — YAML `functions:` → driver-backed template fns
- Snippets — `snippets:` table, `snippet:` ref/inheritance,
  `${snippet(name)}`
- Strict mode — preflight (`on`/`emits`/event schemas), audit pass
- Safety filters — capability checker, schema validator,
  sanitized child env
- State store — `state.key`/`skip_if` scoped store (memory impl)

### Build (not yet existing — net-new or deferred)

- **Scope resolver** — never existed in v1 (tests assert pass-through);
  spec now at `docs/reference/scope-resolution.md` — I-009
- Durable store — durable `EventStore`/`StateStore` adapters
- Queue-supersede policy — event→key dedup, active-run cancel, drain — I-010
- `driver-git` + ingress/egress drivers — ADR-0080 surfaces
- Source port + bundle store — ADR-0079 distribution
- MCP surface — campaign/engine over MCP
- `.driver.yml` / `.field.yml` mechanisms — ADR-0026/0032/0033,
  ship-or-drop — I-011
- Ingress injection channel — `LocalBus::publish` is `&mut` — I-014
- `OtelObserver` bridge — observer port → OTEL
- `**` ADR-0015 text correction — ADR-0015 still says "replace
  `mode: all`"; `mode: all` exists and stays
- Duplicate-id dedupe on merge — include/discovery merges can
  produce duplicate node ids; needs dedupe ruling

### Fix (rulings needed on ported semantics)

- `emits` glob ruling — strict check is exact-match in v1, which
  blocks wildcard/shard emits under strict — I-012
- `max_fires: 0` under strict — falls back to the default in both
  v1 and the port (aspirational there too); strict should reject it

