# Architecture

Peregrin is the *policy* layer for coding-agent sessions and workflows:
who may act on a session, when, and on which machine — plus the
orchestration plane (campaigns, drivers, events, transport) on top.
Session *mechanism* — attach, prompt, restore, per-agent stores — lives
in [sepia](https://9aia.com/sepia/), which peregrin consumes over a
subprocess wire rather than by linking internals.

```
                 ┌─────────────── peregrin-hub ───────────────┐
                 │  hypha-hub gateway · sync · arbitration ·    │
                 │  fleet dispatch (headless — no UI on main)   │
                 └──────────────┬───────────────┬─────────────┘
                        HTTP+SSE │               │ HTTP+SSE
                  ┌──────────────▼──┐    ┌───────▼──────────┐
                  │ peregrin-node A │    │ peregrin-node B  │
                  │  session routes │    │  session routes  │
                  │  + entity plane │    │  + entity plane  │
                  └───────┬─────────┘    └────────┬─────────┘
                  spawn + │ ndjson JSON-RPC       │
                  ┌───────▼─────────┐    ┌────────▼─────────┐
                  │   sepia serve   │    │   sepia serve    │
                  │  session plane  │    │  session plane   │
                  └─────────────────┘    └──────────────────┘
```

## Mechanism vs policy

The split is deliberate: sepia does things *to* sessions; peregrin
decides *who/when/where*. Concretely — `hypha-control` (the extracted substrate, a git
dep of peregrin) holds the
lock registry, takeover arbitration, and `pick_owner` routing; the hub
consults it before forwarding; nodes report lock state up. Anything
that mutates a session (prompt, attach, rewind) flows down to sepia
with the policy decision already made.

## The sepia boundary

A node never links `sepia-session` into orchestration code. It spawns
`sepia serve` and drives it over ndjson JSON-RPC stdio — the same
framing sepia's own store drivers speak — via
`peregrin-session::ServicePlane`. An in-process `LocalPlane` remains
selectable for dev (`PEREGRIN_SESSION_PLANE=local`/`inprocess`); both
implement the same `SessionPlane` trait, so `peregrin-http` and
everything above it is
blind to which side of the fork it's on. The manifest handshake
negotiates protocol versions at spawn.

## Planes

- **Session plane** — the sepia surface: sessions, agents, attach,
  prompts, events. Nodes own it; the hub proxies it.
- **Entity plane (substrate)** — the generic `hypha-*` layer:
  `Entity {id, kind, indexDoc}` + namespaced ops + opaque snapshots,
  the outbox, locks, sync, and the `/api/entities*` + generic routes
  (`/api/events`, `/api/node`, `/api/pair`, `/api/auth/*`,
  `/api/push/*`, `/api/fs`, `/api/user`, `/api/health`). Hypha never
  names an integration's type; peregrin registers its kinds.
- **Integration planes** — peregrin's own: sessions register as
  `kind="sepia-session"` entities (via the `sepia-peregrin` adapter
  in the sepia repo), campaigns as `kind="campaign"` entities
  (`campaign.start|pause|resume|cancel|signal` ops, `campaign
  serve` hosting, `runs_on` fleet scheduling). Campaign *workflows*
  run on `peregrin-engine` + loaders + templates — the event bus,
  remote transport, and the outbox for offline ordering.
- **App plane** — `peregrin-http` composes the session routes with
  hypha's generic routes under one listener; `peregrin-hub` is a
  thin `hypha-hub` gateway (merged entity views, sync, arbitration,
  fleet dispatch). Node-local overlay state (`PEREGRIN_META` — pins,
  archives, run spans) is node-owned, not part of the session wire.
  The Leptos web UI is experimental and lives on the
  `experimental/web` branch — `main` is headless.

## Ports and adapters

Policy crates are pure; every I/O seam is a port
(`peregrin-{store,bus,remote,config,secret,service,loader,template}`)
with impls behind it (`-toml`, `-http`, `-yaml`, `-keyring`, `-dotenv`,
`-systemd`). Engine semantics are documented in `project/adrs/` —
read those before changing campaign behaviour.
