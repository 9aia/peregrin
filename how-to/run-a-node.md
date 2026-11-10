# Run a node

This guide shows how to run `peregrin serve` — the machine daemon that
hosts the session plane and answers `/api/*` for the hub and CLI.

## Basics

```bash
peregrin serve
```

Binds `PEREGRIN_HOST:PEREGRIN_PORT` — default `127.0.0.1:18787`
(`PEREGRIN_PORT` falls back to the generic `PORT`). Check it:

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

Clients exchange a short-lived code for a bearer credential:

```bash
peregrin pair      # prints a code on the node
peregrin redeem <code>   # trades it for a token
```

`/api/pair` is rate-limited — `PEREGRIN_PAIR_LIMIT_PER_MIN` /
`_PER_DAY`.

## The session plane

By default the node runs the session plane as a managed subprocess —
it spawns `sepia serve`, handshakes `manifest`, and respawns it once
on death. `SEPIA_*` env vars (driver dirs, `SEPIA_DEVIN_DB`, lock
TTLs) are read by the sepia child — export them on the node.
`PEREGRIN_SEPIA` overrides binary discovery when needed.

```bash
export PEREGRIN_SEPIA=/path/to/sepia   # optional override for discovery
```

For dev you can link sepia in-process instead (the `LocalPlane` test
mode) — same `SessionPlane` surface, no subprocess:

```bash
export PEREGRIN_SESSION_PLANE=local    # or: inprocess
```

## Node identity and state

- `PEREGRIN_HOME` — node data dir (`~/.local/share/peregrin` default):
  `node.json`, `meta.json`, `tokens.json`.
- `PEREGRIN_NODE` — override the identity file path.
- `PEREGRIN_NAME` — display name for `GET /api/node` and the hub.
- `PEREGRIN_META` / `PEREGRIN_META_PATH` — the overlay store file
  (pins, archives, spans).

## systemd

Service lifecycle is **tamias**'s job — peregrin ships the provider
(`tamias-driver-peregrin`), tamias renders and owns the units. Deploy a
node through `tamias install`/`deploy` of a package that declares the
peregrin provider and `provider.services[]`; tamias writes the unit,
enables it, and owns start/stop/restart/logs. The hidden
`peregrin provision {install,deploy,uninstall}` verbs are the provider
entrypoints tamias shells out to — they generate node config and never
write service units themselves.

## Ephemeral nodes (CI runners)

`peregrin serve --ephemeral` (or `PEREGRIN_EPHEMERAL=1`) runs the node
as an elastic worker: SIGTERM-aware shutdown plus a snapshot flush of
live/locked entities on exit. See
[Run a CI runner node](run-a-ci-node.md).
