+++
id = "P-002"
title = "Entity plane vs orchestration plane: keep one repo, enforce dep direction; orchestration becomes an entity-plane integration"
status = "open"
created = "2026-11-09"
owner = "devin"
priority = "medium"
labels = ["architecture", "peregrin"]
+++

## Context

Peregrin carries two planes that are conceptually distinct but currently ship in one repo/binary:

- **Entity plane** (distributed state/ownership substrate): `peregrin-node-api` (EntityApi contract), `peregrin-sync` (index projection), `peregrin-outbox` (per-entity op queue), `peregrin-control` (locks, takeover, merge/route), `peregrin-push`, `peregrin-hub` (aggregation gateway), `/api/entities*` on `peregrin-http`.
- **Orchestration plane** (event-driven campaigns): `peregrin-core`/`peregrin-engine`/`peregrin-driver-*`/`peregrin-loader*`/`peregrin-template`/`peregrin-bus`/`peregrin-queue`.

Question raised: can the entity plane be split from campaigns, or is it intrinsically orchestration?

## Findings (dep-graph audit, 2026-11-09)

- Orchestration crates (`engine`, `core`, `node`, `cli`, `mcp`) have **zero deps** on entity crates — they only use ports (`store`, `bus`, `secret`, `loader`, `template`, `driver`).
- `peregrin-node-api` has no internal deps (pure contract); `outbox`/`control`/`push` are self-contained leaves.
- The **only** consumer of the whole entity plane is `peregrin-hub` (the aggregator).

So the split already exists at crate level. The real coupling is:

1. **Deployment unit** — `peregrin serve` hosts both planes.
2. **Shared ports** — `store`, `bus`, `remote`, `http`, `auth/pairing` impls serve both.
3. **Hub** — already the natural seam; aggregates both surfaces today.

## Position

The entity plane is not intrinsically orchestration — it is a small distributed-state substrate (ownership + op delivery + snapshots + projections; a mini-etcd-with-queue). Campaigns are one *producer of ops*; sepia is another. The end-state worth aiming at is the reverse of a split: **orchestration becomes an integration on the entity plane**, the same way sepia is — campaign runs modeled as entities (`kind="campaign-run"`), driven by namespaced ops (`campaign.pause`, `campaign.resume`), snapshotted for migration.

## Proposal

Keep one repo; enforce clean dep direction:

```
orchestration (engine/drivers/loaders) → entity plane (node-api/sync/outbox/control/push) → ports (store/bus/remote/http/auth)
```

Concretely, after PL-004 lands:

1. Factor a **node runtime** layer out of `peregrin-node` — transport, auth/pairing, port wiring — that both planes plug into.
2. Make the entity plane the substrate contract; let campaign runs register as an entity kind rather than a parallel run-state system (evaluate whether `peregrin-queue` run state migrates to entities or stays engine-internal).
3. Keep `peregrin-hub` as the only cross-plane aggregator; prohibit orchestration crates from importing entity crates (arch lint).
4. Revisit repo/binary split only if a real second consumer of the entity plane appears (e.g. a non-peregrin workload host) — premature now.

## Open questions

- Do campaign runs need entity semantics (locks, snapshots, takeover) or is engine-internal run state sufficient? If the latter, the planes stay peers and the dep rule is simply "neither imports the other; hub composes".
- Does `peregrin-bus`/event feed belong to the entity plane (push notifications) or orchestration? Today push consumes session-lifecycle events — post-ADR-0091 this becomes a generic event→notification map owned by the plane.
