# HTTP API

The node serves REST + SSE under `/api/*`. `peregrin` (the CLI) and
`peregrin-hub` both consume this surface — everything the UI can do is
on this wire.

## Identity and health

| Route | Purpose |
| --- | --- |
| `GET /api/health` | Liveness + store readability |
| `GET /api/node` | Node identity, agents, capabilities |
| `PATCH /api/node` | Rename the node (persisted to `PEREGRIN_NODE`) |
| `GET /api/user` | OS user the node runs as |
| `GET /api/agents` | Agent runtimes registered on the plane |
| `GET /api/fs` | Subdirectory listing for a path |
| `GET /api/events` | SSE feed — session/meta/project/heartbeat |
| `POST /api/pair` | Redeem a pair code for a bearer token |

## Sessions

| Route | Purpose |
| --- | --- |
| `GET /api/sessions` | Merged list (`?locks=1` probes live holders) |
| `GET /api/sessions/:id` | Session detail or summary |
| `GET /api/sessions/:id/history` | Paginated history |
| `GET /api/sessions/:id/export` | Session IR JSON |
| `POST /api/sessions` | Create (`cwd`, `agent`, `title`, `model`) |
| `POST /api/sessions/import` | Import IR or `history` array → store |
| `POST /api/sessions/:id/prompt` | Send a turn to an attached session |
| `POST /api/sessions/:id/attach` | Attach (`takeover` preempts live holder) |
| `POST /api/sessions/:id/detach` | Release a held session |
| `POST /api/sessions/:id/cancel` | Cancel a running turn |
| `POST /api/sessions/:id/permission` | Resolve a permission request |
| `POST /api/sessions/:id/convert` | Copy into another agent's store |
| `DELETE /api/sessions/:id` | Delete from the owning store |
| `PATCH /api/sessions/:id` | Rename / pin / archive (overlay meta) |

## Projects and config

| Route | Purpose |
| --- | --- |
| `GET /api/projects` | Node-local project grouping of sessions |
| `GET /api/config` | Agent config IR (rules, skills, hooks, MCP) |

## Auth and errors

Non-loopback nodes require `Authorization: Bearer <PEREGRIN_TOKEN>`.
Errors are JSON `{ "code", "message", "cause" }` mirroring
`ControlErrorCode` — `not_found`, `locked` (a live holder blocks the
op; pass `takeover`), `invalid_params`, `internal`.
