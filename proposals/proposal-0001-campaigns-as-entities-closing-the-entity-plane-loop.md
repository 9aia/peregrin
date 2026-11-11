+++
id = "P-001"
title = "Campaigns as entities — closing the entity-plane loop"
status = "done"
created = "2026-11-05"
priority = "med"
+++

# Campaigns as entities — closing the entity-plane loop

Status: **draft** · Created: 2026-11-05

## Goal & Overview

ADR-0091's payoff: a campaign run becomes `kind="campaign"` on the
entity plane, so multi-node sync, the durable outbox, lock
arbitration, and snapshot transfer apply to runs with zero new
distributed machinery. Today `peregrin campaign run` is node-local —
`peregrin_node::campaign` composes a `Runner` in-process and the log
dies with it (`crates/peregrin-node/src/campaign.rs`). This proposal
makes the *run* an entity: created via `POST /api/entities`, driven
via `campaign.*` ops, observable in the merged index, resumable on a
paired node via snapshot export → import → `--replay`.

The engine is already shaped for this. The event log **is** the run
state (`docs/explanation/events.md`); `RunOptions::replay_events` +
`LocalBus::seed`/`check_ready` reconstruct a run from a prior log
(`crates/peregrin-engine/src/runner.rs`, `peregrin-bus/src/bus.rs`).
Transfer is therefore not a new concept — it is the existing replay
path moved across a node boundary, plus the pieces the log doesn't
carry (armed timers, state-store records).

Non-goal: distributing execution of individual graph *nodes* across
machines (a `shell` node on tower, a `git` node on laptop). That is
the broker-bus future — events would have to cross nodes mid-run.
Here the entity is the run's *control plane*: one runner thread, one
owning node, movable between them.

## Requirements

- Register `kind="campaign"` entities on `peregrin-node`; indexDocs
  and `campaign.*` op payloads are owned by the campaign integration
  (in-tree, like `sepia-peregrin` but peregrin-native), opaque to the
  substrate per ADR-0091. No sepia/session type named.
- `campaign.*` ops ride `POST /api/entities/{id}/ops`; node down →
  op queues in the hub outbox → drains in FIFO on reconnect.
- `GET /api/entities/{id}/export` yields kind-shaped snapshot bytes;
  `POST /api/entities/import?node=` resumes the run on the target.
- A running campaign holds its entity lock on the owning node; the
  merged index reflects state transitions within one sync interval.
- `peregrin campaign run` (local, in-process) keeps working
  unchanged — the entity surface is additive.
- `cargo xtask check` + `xtask test` green at each phase (PL-004
  phase 6 gate).

## Technical Design

### Entity identity — the entity is the *run*

```json
{ "id": "run-01J…", "kind": "campaign", "node": "tower",
  "indexDoc": { … }, "locked": true, "live": true }
```

The entity models a *run*, not the campaign definition. Arguments:

- The entity plane tracks **state, ownership, and lifetime** —
  exactly what a run has (`queued → running → done|failed|cancelled`)
  and a definition does not. A definition is content, not workload.
- Lock/snapshot/takeover semantics attach to the executing instance;
  snapshotting "the ci-gate campaign" means nothing without a log.
- Campaign *definitions* are already addressable content (bundle
  refs via ADR-0079, file paths, inline YAML) — they belong inside
  the start payload, not in the entity plane.
- A `kind="campaign-def"` kind (grouping runs, scheduling) can be
  layered later; starting with it buys a second kind for an
  indexDoc-only need that `indexDoc.name` already serves.

### indexDoc — the list-view projection

Rebuilt by folding the event log; updated on each lifecycle beat
(start, first fire, terminal event, sync tick):

```json
{
  "name": "ci-gate",
  "state": "running",                    // queued|running|done|failed|cancelled
  "campaignRef": { "kind": "bundle", "ref": "libraries/ci-gate@0.3.0" },
  "startedAt": "2026-11-05T14:02:11Z",
  "finishedAt": null,
  "counts": { "events": 412, "fires": 37, "failed": 0 },
  "pendingTimers": 2,
  "logRef": "runs/run-01J…/log.jsonl"
}
```

`state` derives from the log tail: no terminal event → `running`;
`workflow.done`/`failed`/`cancelled` → the matching state. The fold
is pure over `Vec<Event>` — trivially testable.

### Ops — `campaign.*`

Op kinds are namespaced per ADR-0091; the substrate routes
`{entity, kind, payload}` to the owner and never interprets bodies.

| op | valid in | effect |
| --- | --- | --- |
| `campaign.start` | `queued` only (entity created via `POST /api/entities` with `kind="campaign"`) | loads the campaign, spawns the run under a `RunSupervisor` slot |
| `campaign.cancel` | `running` | injects `Event::cancel(reason)` into the run's ingress channel — the cancellation surface the runner honours at the next loop iteration |
| `campaign.signal` | `running` | injects an arbitrary `Event` into ingress — ingress drivers, human-input replies, external triggers all reuse this one verb |
| `entity.delete` | any (substrate op) | cancel-if-running, then remove; refuses while `running` unless `payload.force` |

```json
// POST /api/entities/run-01J…/ops
{ "kind": "campaign.start",
  "payload": {
    "campaign": { "ref": "bundle:libraries/ci-gate@0.3.0" },
    "params": { "config": { "repos": ["9aia/peregrin"] } },
    "dryRun": false } }
```

`campaign` alternates by value (`{"yaml": "name: …"}`), by ref
(`{"ref": "bundle:…"}` resolved via `peregrin-source`), or by path
(`{"path": "/abs/x.yaml"}` — same-node only). **Secrets never travel
ops or snapshots** — `${secret.*}` resolves against the *target*
node's env/keyring/env-file chain (`default_secret_stores`), which
is already node-local.

`stop`/`pause`/`resume` are deliberately absent: `stop` is
`cancel` with a different label; `pause`/`resume` = snapshot →
import and are covered by the transfer path, not by live ops.

**Validity while queued vs running.** `campaign.start` on a
`running` entity is a 409; `cancel`/`signal` on a `queued` entity
no-op into the outbox (they drain before `start` if ordered after
it — per-`(node,entity)` FIFO in `peregrin-outbox` preserves this).

**Outbox semantics.** `campaign.*` ops are `OpKind::Turn` —
dead-letter on replay failure rather than reorder into a live run
(a `cancel` arriving after `done` must die, not resurrect).
`entity.delete` is `OpKind::Metadata`, last-writer-wins.

### Where the engine lives

The node hosts a **campaign plane**: the kind-registration half of
`peregrin_node::campaign` extracted into a `peregrin-campaign` crate
(working name — PL-004 leaves identifiers provisional) exposing the
integration hooks: `index_doc(log) → Value`, `handle_op(entity, op)`,
`snapshot_export(run) → bytes`, `snapshot_import(bytes) → run`.
`peregrin-http`'s `entities.rs` (already marked INTERIM for
kind-dispatch) gains a kind registry instead of hardcoding
`sepia-session`.

`campaign.start` spawns the run on the node's `RunSupervisor` (one
active run per slot, supersede policy intact) with
`runner_with_log_ingress` — the durable JSONL log is already the
snapshot's spine. Run completion folds the log into the indexDoc
(`state`, `counts`, `finishedAt`) and emits a lifecycle beat into
the entity feed the sync projection picks up.

### Snapshots / transfer

```yaml
format: campaign-snapshot/v1
entity: { id: run-01J…, kind: campaign }
campaign: <serialized Campaign>          # by value; ref kept in annotations for provenance
log: [ <Event>, … ]                      # the JSONL log verbatim — the replay seed
pendingTimers:                           # the deadline-heap drain, absolute wall-clock
  - { event: <Event>, atEpochMs: 17308…, holdsRun: true }
state: { <state.key records> }           # StateStore::to_record, may be {}
counters: { retryCounts: {}, timerSeq: 41 }   # best-effort fidelity
```

Import path: load `campaign` → `Runner` with `replay_events = log`
(the `seed`+`check_ready` path reconstructs readiness, fire counts,
`max_fires` eligibility) → re-arm `pendingTimers` as
`Instant::now() + (atEpochMs − now)` (past deadlines fire
immediately) → restore the state store → resume the loop.

**Timers.** `delay`/`schedule` emissions sit on
`BinaryHeap<ScheduledTick>` keyed by `Instant` — heap state, not
log state. The snapshot carries them explicitly as `pendingTimers`
with absolute epoch-ms deadlines; this needs a small engine seam
(`Runner::pending_timers()` / `arm_timers()`), not new semantics.
Two refinements:

- *Poll ticks* (`holds_run: false`) need no snapshotting — they
  re-arm on resume from `node.poll.interval`, matching the
  sleep-race semantics (a tick only matters if the run is alive).
- *Clock domains.* `atEpochMs` is wall-clock, so a snapshot moved
  between machines stays correct; `Instant` never crosses the wire.

An alternative — reconstructing timers from the `*.delayed` /
`*.scheduled` arm events already in the log — was rejected: it
couples substrate import to driver-specific emit shapes and misses
`passthrough` payloads. Explicit `pendingTimers` is generic.

### Locks / arbitration

A `running` campaign entity holds the RW lock on its owning node —
`campaign.*` ops route through `Ownership::route` and contend like
any other entity. **There is no live takeover**: a running runner
can't be preempted mid-flight without a snapshot, so `Preempt` on a
live `campaign` entity means *export → import → resume elsewhere*
(orchestrated transfer), not in-place hijack. A stale lock (node
dropped mid-run) leaves the entity `running`-stale; takeover exports
the last durable log checkpoint and replays — events between the
last flush and the crash are lost, same as a local crash today.

### Migration / coexistence

- `peregrin campaign run` — untouched: same `peregrin_node::campaign`
  composition, in-process, ephemeral.
- `peregrin campaign start <yaml> [--node n]` — new verb over
  `EntityApi`: `create_entity("campaign")` → `campaign.start` op.
  `--replay <log>` maps to `snapshot_import` + start.
- Rollout: kind registration lands behind the entity registry in
  `entities.rs`; the substrate needs no `campaign`-named code beyond
  the kind string constant.

## Open questions

- **Run-entity vs campaign-entity granularity** — recommended: entity
  = run, definition rides in the payload. A second
  `kind="campaign-def"` (grouping, scheduling, fan-out of runs) is
  deferred; confirm one kind suffices for v1.
- **Timer persistence across transfer** — recommended: explicit
  `pendingTimers` with wall-clock deadlines in the snapshot. Open:
  are sub-second `delay`s worth preserving exactly, or is
  "re-arm with remaining wall time, past-due fires immediately" the
  right semantics on import? (Chosen semantics stated; validate in
  review.)
- **Campaign by value vs by ref in ops/snapshots** — inline YAML is
  self-contained but heavy on the wire and divergent on edit; a
  bundle ref is light but requires the target to resolve it.
  Recommended: snapshot carries the *resolved* campaign by value
  (exact reproduction) while `campaign.start` accepts either.
- **Snapshot format versioning** — `format: campaign-snapshot/v1`
  proposed; the substrate moves bytes opaquely so versioning is the
  integration's problem — but log/`Event` shape changes will break
  replay; pin the snapshot to the event-envelope contract version?
- **Run state on node crash without transfer** — the JSONL log is
  durable but `pendingTimers`/`state` flush only at snapshot-export;
  is export-on-checkpoint-interval (crash recovery within a node)
  worth the I/O, or is best-effort loss acceptable like today?
- **`campaign.signal` scope** — one generic inject verb vs named
  verbs (`campaign.human.reply`, …). Generic keeps the op surface
  minimal; named verbs give the outbox/UI better semantics. Leaning
  generic + payload-typed.
