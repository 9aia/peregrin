+++
id = "PL-004"
title = "Entity-plane restructure — de-chat-shape peregrin's distributed layer"
status = "open"
created = "2026-11-05"
owner = "devin"
priority = "high"
+++

# Entity-plane restructure — de-chat-shape peregrin's distributed layer

Implements ADR-0091. Turns the session-shaped distributed plane (node-api, sync, outbox, control, push, hub) into a generic entity plane; sepia sessions become a `sepia-peregrin` integration crate living in the sepia repo.

Invariants throughout: `cargo xtask check` + `xtask test` green on main at every phase end; no peregrin crate names a sepia/session type (grep gate added in phase 1); `experimental/web` untouched until phase 5.

## Phase 0 — spec freeze

- [ ] ADR-0091 accepted (this doc + review).
- [ ] `docs/` or `project/notes/` spec for the entity model: `Entity{id,kind,node,indexDoc,health}`, `Op{entity,kind,payload}`, `Snapshot{entity,ref,bytes}`; op-kind namespacing (`<kind>.<verb>`); index-doc ownership rules (integration writes, substrate stores/opaque-reads).
- [ ] Decide projection schema version + migration policy (fresh DB vs migrate; projection is a cache → drop-and-resync is acceptable).

Gate: ADR accepted, spec written.

## Phase 1 — `peregrin-node-api` → entity contract

- [ ] Replace `NodeApi` session surface with entity surface: `list_entities(kind?)`, `entity(id)`, `submit_op(entity, Op)`, `entity_index(node)`, `snapshot_export(id)`, `snapshot_import(bytes)`, `health()`.
- [ ] Move session DTOs (`SessionSummaryDto`, history, checkpoints, capabilities, transfer) out of the crate — staging home: `sepia-peregrin` protocol module (defined now, published from sepia repo in phase 4).
- [ ] `HttpNodeApi` + `FakeNodeApi` reworked over entities.
- [ ] Add `xtask check` gate: no `Session*Dto`/`session`-named type inside `peregrin-*` substrate crates (allowlist `peregrin-session` boundary crate + tests).

Gate: workspace compiles; hub builds against entity contract with a temporary in-crate session→entity shim (removed in phase 3/4).

## Phase 2 — `peregrin-outbox` + `peregrin-control` genericized

- [ ] Outbox entries become `{entity_id, op_kind, payload}` — `op_kind` is `<kind>.<verb>` namespaced; drop chat-verb enums.
- [ ] `peregrin-control`: lock registry keyed on `EntityId` not session id; `Actor`/`Decision`/`takeover`/`merge`/`route` text-generic.
- [ ] Tests renamed/rewritten in entity terms.

Gate: crates compile + unit tests green in entity vocabulary (hub still shimmed).

## Phase 3 — `peregrin-sync` + `peregrin-hub` → entity index/gateway

- [ ] Projection schema v2: `entities(node, id, kind, index_doc, health, updated_at)` + `nodes` table unchanged in spirit; drop-and-resync migration (projection is a cache).
- [ ] `SyncEngine`/`NodeClient` sync entity indexes + op acks, not session rows.
- [ ] `SyncNodeApi` → `SyncEntityApi` implementing the phase-1 trait: merged index reads, op submit→outbox when node down, snapshot routing via session_router equivalent (`entity_router`).
- [ ] hub HTTP surface: `/api/*` passthrough already generic; entity index/op endpoints exposed under a substrate prefix (no `/hub/*` session routes — the UI's transport lives in the sepia integration now).
- [ ] `peregrin-push` genericized: `(kind,title,body,url)` events; session→push rules deleted from hub (re-implemented integration-side in phase 4 if wanted).

Gate: hub boots multi-node, entity ops round-trip incl. offline queueing; `xtask check` + affected suites green.

## Phase 4 — `sepia-peregrin` adapter (sepia repo)

- [ ] New crate in the sepia workspace (public API = peregrin's entity trait + DTO-free transport): `kind="sepia-session"` registration, snapshot via existing session IR export/import, op translation (`sepia.prompt`, `sepia.attach`, `sepia.rewind`, `sepia.permission`…), indexDoc = summary projection for list views, session-shaped DTO surface for clients.
- [ ] Driver/bridge so `peregrin-node`/`peregrin-hub` discover and serve sepia sessions through the entity plane (adapter registration point on the node side; config/`PEREGRIN_NODES` semantics unchanged).
- [ ] Optional: session→push rule set (done/permission notifications) as integration-layer config.

Gate: sepia repo tests green; peregrin e2e: create/prompt/rewind a session through the entity plane end-to-end.

## Phase 5 — session UI back onto the plane (`experimental/web`)

- [ ] Web crates consume `sepia-peregrin`'s client surface (direct HTTP or a thin client lib) — no peregrin session types.
- [ ] Hub on `experimental/web` serves the UI + integration routes; verify SSR/hydrate path, `xtask site`/`smoke`/`e2e` on that branch.
- [ ] Merge-back criteria documented: UI stable + entity contract stable → candidate to re-land web on main.

Gate: branch green; UI feature-parity with pre-split behavior.

## Phase 6 — campaigns as entities

- [ ] Campaign state modeled as entities (`kind="campaign"`): multi-node campaign sync/outbox/transfer rides the same machinery — the original thesis closes the loop.
- [ ] Docs (AGENTS.md, README, project/adrs references) updated to entity vocabulary.

Gate: campaign ops observable in the merged index; docs consistent.

## Risks / notes

- Largest blast radius is phase 3 (sync schema + hub). Phase 1's allowlist gate prevents session-type leakage back in.
- `experimental/web` divergence grows until phase 5 — keep main merges into it minimal (revert-commit model already supports this).
- Names are provisional (`Entity`, `sepia-peregrin`); ADR-0091 is normative on responsibilities, not identifiers.
