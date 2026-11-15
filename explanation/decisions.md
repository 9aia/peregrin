# Decisions

ADR-lite: the load-bearing decisions, each with the alternative it
beat and the consequence it carries. The full history — dozens of
ADRs — lives in `project/adrs/` in the repo; read those before
changing engine semantics.

## Policy over mechanism

**Decision:** peregrin decides *who/when/where* a session action
happens; sepia does things *to* sessions. The node spawns
`sepia serve` and drives it over ndjson JSON-RPC — no `sepia-session`
type is ever linked into orchestration code.

- *Alternative:* fold session mechanism into peregrin, or have
  peregrin link sepia internals.
- *Consequence:* the boundary is a wire protocol with a manifest
  handshake; either side can rev independently. Cost: a subprocess
  hop on every session op, and `LocalPlane` exists only for dev.

## Entities as the integration seam

**Decision:** sessions and campaigns register as generic
`{id,kind,indexDoc}` entities with namespaced ops and opaque
snapshots on hypha's substrate; peregrin never names a sepia type in
substrate code (`project/adrs/0091` is the contract).

- *Alternative:* bespoke session/campaign tables inside the node.
- *Consequence:* one sync/lock/outbox machinery serves both planes;
  new kinds are registrations, not migrations.

## Latest-wins queue

**Decision:** runs on the same `{repo}:{ref}` key supersede — a newer
push cancels the in-flight run rather than queueing behind it.

- *Alternative:* FIFO queueing of every event.
- *Consequence:* stale work never lands, at the cost of needing
  explicit cancellation plumbing through drivers.

## Capabilities, not ambient trust

**Decision:** campaigns declare `capabilities:`; secrets resolve by
NAME through a per-campaign grant (`capabilities.secrets`), and an
unresolvable name fails the node.

- *Alternative:* inherit the operator's full environment.
- *Consequence:* AI-governed workflows are bounded by construction —
  see [Safety](safety.md) and [Threat model](threat-model.md).

## YAML campaigns as plugins

**Decision:** workflows are YAML files distributed as tamias bundles
(`peregrin-bundle`), with `include`/snippet resolution for
composition — no compiled workflow format.

- *Alternative:* a typed workflow DSL compiled to Rust.
- *Consequence:* campaigns are data, reviewable in diffs and editable
  without a toolchain; validation moves to `campaign validate` and
the loader. See [Plugins](plugins.md).
