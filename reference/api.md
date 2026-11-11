# HTTP API

`peregrin serve` composes two route sets under one listener: the
**session routes** (peregrin's integration surface) and the **generic
entity routes** (the hypha substrate — `/api/entities*`, pairing,
auth, push, fs). `peregrin-hub` exposes the same generic surface
merged across nodes, plus `/api/node-status`.

## Identity and health

| Route | Purpose |
| --- | --- |
| `GET /api/health` | Liveness + store readability |
| `GET /api/node` | Node identity, agents, capabilities (incl. `PEREGRIN_RUNNER_LABELS`) |
| `PATCH /api/node` | Rename the node (persisted to `PEREGRIN_NODE`) |
| `GET /api/user` | OS user the node runs as |
| `GET /api/fs` | Subdirectory listing for a path |
| `GET /api/events` | SSE feed — session/meta/project/heartbeat |
| `POST /api/events/inject` | Authenticated typed-event ingest into running campaigns — see below |
| `GET /api/agents` | Agent runtimes registered on the plane |
| `GET /api/client/keypair` | Client keypair for the entity plane |

## Pairing and auth

| Route | Purpose |
| --- | --- |
| `POST /api/pair` | Trade a one-time code for a bearer credential (`sepia_…`); rate-limited |
| `POST /api/auth/login` / `POST /api/auth/logout` | Cookie session flow |
| `GET /api/push/vapid` / `POST /api/push/subscribe` | Web-push keys and subscriptions |

`Authorization: Bearer <token>` guards non-loopback binds;
`?access_token=` works only on the SSE paths.

## Entities (generic substrate)

Kind-agnostic entity plane — sessions register as
`kind="sepia-session"`, campaigns as `kind="campaign"`. Op kinds are
namespaced (`sepia.prompt`, `campaign.start`, `entity.delete`);
snapshot bytes are opaque end-to-end.

| Route | Purpose |
| --- | --- |
| `GET /api/entities` | Entity index list (`?kind=` filters) |
| `POST /api/entities` | Create an entity (`{id?, kind, indexDoc}`) |
| `GET /api/entities/snapshot` | Bulk snapshot export |
| `POST /api/entities/snapshot` | Snapshot import (kind-shaped bytes, `?node=` on hubs) |
| `GET /api/entities/{id}` | Entity detail |
| `DELETE /api/entities/{id}` | Delete (`entity.delete` op) |
| `POST /api/entities/{id}/ops` | Submit a namespaced op (`{op, params}`) |
| `GET /api/entities/{id}/snapshot` | Single-entity snapshot export |
| `GET /api/entity-index` | Projection index document |
| `GET /api/pending-ops` | Durable outbox depth (hub: real; bare node: empty) |

## Event ingest

`POST /api/events/inject` is the authenticated write half of the
event surface — external producers (the bupha CI worker's
`ci.job.completed` emit, forge check runners, ops tooling) inject a
typed event into the campaign engine. `GET /api/events` stays the
read-only SSE feed.

- **Auth** — the shared bearer gate: `Authorization: Bearer
  <PEREGRIN_TOKEN>` (or a paired `sepia_…` credential /
  `sepia_token` cookie); open on tokenless loopback like every other
  `/api/*` route.
- **Body** — `application/json`, ≤ 64 KiB:

  ```json
  {
    "type": "ci.job.completed",
    "payload": { "job": "build", "result": "success" },
    "idempotencyKey": "job-1:success",
    "campaign": "camp-entity-id",
    "source": "bupha"
  }
  ```

  `type` is required and non-empty (`kind` accepted as the `Event`
  wire alias); `payload` defaults to `{}`; `source` defaults to
  `"api"`; `idempotencyKey` (`idempotency_key` alias) dedups retries
  against a bounded ring — a replayed key answers
  `{"ok": true, "deduplicated": true}` without re-delivering;
  `campaign` scopes the fan-out to one `kind="campaign"` entity.
- **Fan-out** — the event is cloned into the ingress channel of
  every *running* campaign entity (or the one named by `campaign`).
  Stale/finished runs and closed channels are skipped.
- **Responses** — `200 {"ok": true, "delivered": <runs that
  accepted>}` (`delivered: 0` = accepted, but no campaign entity was
  running); `200 {"ok": true, "deduplicated": true}` on a replayed
  key; `400` malformed body / missing `type`; `401` bad credential;
  `404` unknown scoped campaign; `413` over the body cap; `500` sink
  failure; `501` when the node has no campaign plane wired.

## Sessions

| Route | Purpose |
| --- | --- |
| `GET /api/sessions` | Merged list (`?locks=1` probes live holders) |
| `GET /api/sessions/:id` | Session detail or summary |
| `GET /api/sessions/:id/meta` | Overlay meta (title/pin/projects/spans) |
| `GET /api/sessions/:id/history` | Paginated history |
| `GET /api/sessions/:id/checkpoints` | Checkpoint refs |
| `GET /api/sessions/:id/export` | Session IR JSON |
| `GET /api/sessions/:id/stream` | SSE stream for one session |
| `GET /api/sessions/:id/permissions` | Pending permission requests |
| `POST /api/sessions` | Create (`cwd`, `agent`, `title`, `model`) |
| `POST /api/sessions/import` | Import IR or `history` array → store |
| `POST /api/sessions/:id/prompt` | Send a turn to an attached session |
| `POST /api/sessions/:id/attach` | Attach (`takeover` preempts live holder) |
| `POST /api/sessions/:id/detach` | Release a held session |
| `POST /api/sessions/:id/cancel` | Cancel a running turn |
| `POST /api/sessions/:id/permission` | Resolve a permission request |
| `POST /api/sessions/:id/restore` | Restore files to a checkpoint |
| `POST /api/sessions/:id/rewind` | Rewind the session tree |
| `POST /api/sessions/:id/convert` | Copy into another agent's store |
| `DELETE /api/sessions/:id` | Delete from the owning store |
| `PATCH /api/sessions/:id` | Rename / pin / archive (overlay meta) |

## Projects and config

| Route | Purpose |
| --- | --- |
| `GET /api/projects` | Node-local project grouping of sessions |
| `GET /api/projects/:id` | One project |
| `GET /api/config` | Agent config IR (rules, skills, hooks, MCP) |
| `GET/PUT /api/config/:key` | One config item |

## Hub-only

| Route | Purpose |
| --- | --- |
| `GET /api/node-status` | Per-node health across the fleet |

The hub also merges `/api/entities*` across nodes and proxies SSE /
passthrough to the owning node.

## Auth and errors

Non-loopback nodes require `Authorization: Bearer <PEREGRIN_TOKEN>`
(hub: `PEREGRIN_HUB_TOKEN`). Session-route errors are JSON
`{ "error", "code" }` — `not_found`, `invalid`, `unknown_agent`,
`locked` (a live holder blocks the op; pass `takeover`), `conflict`,
`busy`, `internal`. Generic entity-route errors use hypha's `{error}`
mapping, not the session shape.
