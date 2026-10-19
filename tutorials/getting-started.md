# Getting started

This tutorial takes you from zero to a running peregrin node with the
web UI talking to it. You will need Rust (`cargo`) and
[sepia](https://9aia.com/sepia/) installed — peregrin drives the session
plane through `sepia serve`.

## Install

```bash
cargo install --path crates/peregrin-cli --bin peregrin
cargo install --path crates/peregrin-cli --bin peregrin-hub
cargo install --path <sepia-checkout>/crates/sepia-cli
```

Or inside a checkout, `cargo xtask install` builds `peregrin`,
`peregrin-hub`, and the web bundle into `~/.local`.

## Run the node

```bash
peregrin serve
```

The node binds `127.0.0.1:8787` by default, discovers sepia drivers on
this machine, and serves the `/api/*` surface plus SSE event feed. In
another terminal:

```bash
peregrin health
peregrin sessions list
```

You should see the node's health row and a merged session list from
every agent store sepia can see.

## The web UI

```bash
peregrin-hub
```

The hub serves the Leptos app (default `:3000`) and aggregates every
node in `PEREGRIN_NODES`. Open it, pair with a code (`peregrin pair`
prints one on the node), and sessions from all your machines merge into
one view — locks, takeover, and routing handled by the control layer.

## Where next

- [Architecture](../explanation/architecture.md) — how nodes, hub, and
  the sepia boundary fit together.
- [Run a node](../how-to/run-a-node.md) — tokens, non-loopback binds,
  systemd.
- [Environment reference](../reference/environment.md) — every
  `PEREGRIN_*` knob.
