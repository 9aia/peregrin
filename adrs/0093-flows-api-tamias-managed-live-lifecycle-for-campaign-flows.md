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
operation. Code review adjustments vs the first draft: no `flows/`
dir (the existing `serve/` dir is the persistence layer), route
ownership is explicit, and reload is drain-then-swap.

1. **Flows API on the existing listener** (`peregrin-node::serve`):
   - `GET /api/flows` — hosted flows + state (`active`/`draining`)
   - `POST /api/flows` — `{name, yaml}`: validate, load, spawn
     supervisor live; `409` on duplicate name
   - `DELETE /api/flows/{name}` — unregister demux routes **first**
     (stop new deliveries), then set the flow's shutdown flag;
     returns `202 {draining: true}` — the join completes in
     background and the registry entry drops
   - `POST /api/flows/{name}/reload` — async drain-then-swap:
     unregister → drain old supervisor → respawn from the flow's
     persisted source yaml. Deliveries inside the drain window are
     dropped (GitHub does not retry push webhooks) — the gap is
     scoped to the one flow, strictly smaller than today's
     all-flow restart.
   All routes behind the existing `PEREGRIN_TOKEN` bearer.

2. **Per-flow shutdown — already plumbed.** `SupervisedOptions.
   shutdown` is a per-supervisor `Option<Arc<AtomicBool>>`; the
   daemon passes the same global flag to every campaign today.
   Per-flow lifecycle = a distinct `Arc` per hosted flow plus a
   registry (`name → {flag, join, state, source}`).

3. **Demux route ownership.** `Route` gains an `owner` field (the
   hosted campaign name), plumbed `serve →
   registry_with_ingress_shared → SharedWebhookDriver → Route`.
   `SharedWebhook::unregister(owner)` removes every route with that
   owner — Vec-removal *is* the refcount: two flows watching the
   same repo keep each other's routes (all-match preserved).
   Route registrations after boot are already live (the demux
   reads the shared map per request), so `POST` needs no listener
   rebind; the listener binds unconditionally in serve mode so a
   webhook flow added to a previously webhook-free fleet still
   receives deliveries.

4. **Durability — no new dir.** `peregrin provision deploy` keeps
   writing the rendered campaign doc to `serve/<name>.yaml` (the
   boot-scanned persistence layer, unchanged) **and** POSTs the
   same doc to `/api/flows` when the daemon is reachable — file
   fallback covers daemon-down deploys (next boot picks it up).
   `uninstall` → `DELETE` + remove the file.

5. **CLI + ops.** `peregrin campaign flows
   list|load|unload|reload` — a thin `ureq` client (already a
   workspace dep) against the API; `cli.flows.*` ops derive on
   `tamias-driver-peregrin`. Token resolution reads
   `~/.config/peregrin/env` for `PEREGRIN_TOKEN` (deploy-time env
   lacks the unit's `env_file`).

6. **Queue semantics.** DELETE/reload drain runs *pending* queue
   entries under the old flow today; a `RunSupervisor::
   clear_pending` (and optionally exposing the `cancel_running`
   surface as `cancel_active`) lets DELETE drop queued triggers
   and shorten the drain window. v1 ships `clear_pending`; fast
   cancel is a follow-up if drain time proves painful.

## Consequences

- Flow install/remove/update becomes live with no service restart;
  other flows' webhooks and in-flight runs are untouched — the
  10s+ all-flow downtime disappears for bundle operations.
- Zombie routes die properly: unregister-on-delete means dead flows
  stop "receiving" deliveries that silently fail inject.
- tamias gains real lifecycle verbs for flows via
  `peregrin provision` — bundle state and daemon state cannot
  silently drift. No tamias-repo changes required.
- New failure modes: the reload delivery gap (documented, scoped),
  drain-then-swap ordering, route-owner plumbing through the
  registry, API auth on a tunnel-exposed socket (existing
  `PEREGRIN_TOKEN` bearer reused).
- Explicitly rejected: per-flow units/ports (proliferates listeners,
  requires webhook/DNS migration per flow), provider-instance
  splitting (fights the shared-listener design), and a separate
  `flows/` managed dir (the `serve/` dir already is one).

## Status

proposed
