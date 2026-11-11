# Getting started

This tutorial takes you from zero to a running peregrin node plus the
hub that aggregates it. You will need
[tamias](https://github.com/9aia/tamias) and
[sepia](https://9aia.com/sepia/) installed — peregrin drives the session
plane through `sepia serve`. Rust (`cargo`) is only needed for the
build-from-checkout path below.

## Install

The `peregrin` command ships as a tamias pack (`libraries/peregrin`)
— there is no cargo bin to `cargo install`. With `tamias` installed:

```bash
tamias install 9aia:peregrin@0.2.0    # org registry (needs TAMIAS_REGISTRY_TOKEN)
```

That deploys the `peregrin` shim plus the `peregrin-node`/
`peregrin-hub` payload bins. From a checkout:

```bash
cargo xtask pack                                        # stage release bins
cargo install --path crates/peregrin-hub               # the hub daemon
tamias install --allow-drivers ./libraries/peregrin    # the peregrin command
```

Or inside a checkout, `cargo xtask install` builds
`peregrin-hub` and `tamias-driver-peregrin` into `~/.local` — pair it
with `cargo xtask pack` + the `tamias install` above for the
`peregrin` command itself.

## Run the node

```bash
peregrin serve
```

The node binds `127.0.0.1:18787` by default, discovers sepia drivers on
this machine, and serves the `/api/*` surface — the session routes plus
the generic entity plane (`/api/entities*`, `/api/events`, `/api/node`,
`/api/pair`, …) — with an SSE event feed. In another terminal:

```bash
peregrin health
peregrin sessions list
```

You should see the node's health row and a merged session list from
every agent store sepia can see.

## Run the hub

```bash
export PEREGRIN_NODES="local=http://127.0.0.1:18787"
peregrin-hub
```

The hub is a headless gateway (default `127.0.0.1:13000`) — it merges
every configured node's entities and session views into one API and
dispatches work across the fleet. There is no bundled web UI on `main`;
the experimental Leptos UI lives on the `experimental/web` branch.
Clients authenticate with `PEREGRIN_HUB_TOKEN` (loopback is open) and
`/api/pair` trades a one-time code for a bearer credential.

## Where next

- [Your first gate](first-gate.md) — install a bundle through tamias,
  deploy a campaign node, register the webhook, watch pushes become
  commit statuses.
- [Architecture](../explanation/architecture.md) — how nodes, hub, and
  the sepia boundary fit together.
- [Run a node](../how-to/run-a-node.md) — tokens, non-loopback binds,
  tamias service units.
- [Environment reference](../reference/environment.md) — every
  `PEREGRIN_*` knob.
