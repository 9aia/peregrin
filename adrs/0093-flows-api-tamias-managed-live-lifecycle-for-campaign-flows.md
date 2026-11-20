+++
id = "ADR-0093"
title = "Flows API: tamias-managed live lifecycle for campaign flows"
status = "open"
created = "2026-11-20"
owner = "devin"
priority = "med"
+++

## Context

`peregrin campaign serve <dir>` loads every campaign yaml once at boot
and hosts one supervisor per campaign behind a single shared
`github-webhook` listener. Campaign bundles (`type: peregrin-bundle`)
are installed through tamias, but their deploy step only renders yaml
into `~/.config/peregrin/serve/` — a flat directory the daemon scans
at startup. Consequences:

- Adding, removing, or updating a flow requires restarting the whole
  service — every hosted campaign drains, in-flight runs are exposed,
  and the shared listener drops webhooks (observed as GitHub 502s and
  lost deliveries during the docs-deploy rollout).
- tamias has no lifecycle verb for a *flow*: install/remove/update of
  a bundle does not reach the running daemon, so bundle state and
  daemon state drift (flows exist on disk that were never loaded,
  loaded flows whose bundle was removed).
- The daemon already exposes a management-style route on the same
  socket (`POST /api/events/inject`, bearer `PEREGRIN_TOKEN`), so a
  flows API is an established pattern, not a new surface.

## Decision

Keep the topology — **one `campaign serve` process hosting many
flows** — and make flow lifecycle a first-class, tamias-driven
operation:

1. **Flows API on the existing listener** (`peregrin-node::serve`):
   - `GET /api/flows` — hosted flows + state
   - `POST /api/flows` — `{name, yaml}`: validate, load, spawn
     supervisor, register demux routes + trigger injector
   - `DELETE /api/flows/{name}` — drain that supervisor, unregister
     its webhook routes and injector
   - `POST /api/flows/{name}/reload` — re-read the source yaml and
     swap the supervisor under the campaign's `supersede:` policy
   All routes behind the existing `PEREGRIN_TOKEN` bearer.

2. **Per-flow shutdown.** `SupervisedOptions` gains a per-campaign
   cancel token alongside the daemon-global shutdown flag, so
   `DELETE`/`reload` drains one flow without touching the others.

3. **Demux route add/remove.** `SharedWebhook` gains
   `register`/`unregister` for `params.repos` watches with
   refcounting — two flows watching the same repo must not lose the
   route when one is removed (all-match demux semantics preserved).

4. **Managed flows dir.** API-installed flows persist rendered yaml
   to `~/.config/peregrin/flows/<name>.yaml`; `campaign serve` scans
   `serve/` (base set, unchanged) **plus** `flows/` (managed set) at
   boot. Restart durability with no new storage; duplicate names
   across the union are rejected (existing rule).

5. **CLI + ops.** `peregrin campaign flows
   list|load|unload|reload`; `cli.flows.*` ops derive automatically
   on `tamias-driver-peregrin`.

6. **Bundle lifecycle.** The `peregrin-bundle` tamias driver changes
   its deploy sink: render the concrete campaign yaml (params bound,
   as today) → `POST /api/flows` when the daemon is reachable, else
   write into `flows/` for next boot. `remove` → `DELETE` + delete
   the file. The provider `services:` block is unchanged — still ONE
   `peregrin-campaign.service`.

## Consequences

- Flow install/update/remove becomes live (~1s) with no service
  restart; webhooks keep flowing during bundle operations.
- tamias gains real lifecycle verbs for flows; bundle state and
  daemon state cannot silently drift.
- `serve/` remains the operator-owned base set; `flows/` is
  tamias-owned. Both are plain yaml — debuggable by hand.
- New failure modes to handle: demux unregister refcounting,
  reload-vs-in-flight-run semantics (supersede policy decides),
  name collisions across the two dirs, API auth on a
  tunnel-exposed socket.
- Explicitly rejected: per-flow units/ports (proliferates listeners,
  requires webhook/DNS migration per flow) and provider-instance
  splitting (fights the shared-listener design the engine already
  implements).

## Status

proposed
