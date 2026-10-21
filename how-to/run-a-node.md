# Run a node

This guide shows how to run `peregrin serve` — the machine daemon that
hosts the session plane and answers `/api/*` for the hub and CLI.

## Basics

```bash
peregrin serve
```

Binds `PEREGRIN_HOST:PEREGRIN_PORT` — default `127.0.0.1:8787`. Check it:

```bash
peregrin health
peregrin node       # identity, agents, capabilities
peregrin sessions list
```

## Authentication

Loopback binds need no token. For anything reachable off-machine:

```bash
export PEREGRIN_TOKEN=<shared-secret>
peregrin serve
```

Non-loopback without `PEREGRIN_TOKEN` refuses to start. Clients send it
as a bearer token; the hub uses `PEREGRIN_NODE_TOKEN` per node.

## Pairing

The web UI exchanges a short-lived code for a bearer token:

```bash
peregrin pair      # prints a code on the node
```

Then `peregrin redeem <code>` (or the UI's pair flow) trades it for a
token. `/api/pair` is rate-limited — `PEREGRIN_PAIR_LIMIT_PER_MIN` /
`_PER_DAY`.

## The session plane

By default the node links sepia in-process (`PEREGRIN_SESSION_PLANE` unset
or `inprocess`). To run the session plane as a managed subprocess —
the end-state architecture — set:

```bash
export PEREGRIN_SESSION_PLANE=service
export PEREGRIN_SEPIA=/path/to/sepia   # optional override for discovery
```

The node then spawns `sepia serve`, handshakes `manifest`, and respawns
it once on death. `SEPIA_*` env vars (driver dirs, `SEPIA_DEVIN_DB`, lock
TTLs) are read by the sepia child — export them on the node.

## Node identity and state

- `PEREGRIN_HOME` — node data dir (`~/.local/share/peregrin` default):
  `node.json`, `meta.json`, `tokens.json`.
- `PEREGRIN_NODE` — override the identity file path.
- `PEREGRIN_NAME` — display name for `GET /api/node` and the hub.
- `PEREGRIN_META` / `PEREGRIN_META_PATH` — the overlay store file
  (pins, archives, spans).

## systemd

`peregrin-service` + `peregrin-systemd` are the service port and
adapter — a unit file keeps the node alive and restarts it. `peregrin
service install` writes + enables the unit (`--hub` manages the hub,
`--system` a system unit); `peregrin service disable` stops it and keeps
it stopped across logins (unit file kept) until `peregrin service
enable`. `status`, `restart` and `logs` round out the verbs.
