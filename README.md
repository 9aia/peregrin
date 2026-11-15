# Peregrin

Event-driven orchestration for agent workflows — nodes, campaigns, and
remote transport across your machines. Rust.

Peregrin is the automation platform: a workflow engine for
event-driven, agent-aware campaigns, plus the app that manages
coding-agent sessions everywhere — [sepia](https://github.com/9aia/sepia)
embedded as the session plane on every node.

- **Campaigns** — declarative graphs of events, nodes, and drivers
  (`peregrin-core`/`peregrin-engine`/`peregrin-driver-*`). A node
  listens for events, invokes a driver, emits new events; the engine
  walks the graph until nothing is ready to fire.
- **Nodes** — `peregrin serve` runs a machine daemon: session plane
  (via `sepia`) + campaign drivers + event bus.
- **Hub** — `peregrin-hub` aggregates nodes into one API surface —
  merged entity projection, queued writes, push (a thin
  `hypha-hub` gateway carrying Peregrin's kind policies). The
  experimental web UI lives on the `experimental/web` branch.
- **Ports everywhere** — runner, transport, store, secrets, config,
  service are ports; implementations are separate crates
  (`peregrin-{port}` + `peregrin-{port}-{impl}`).

## Boundary with sepia

sepia owns *mechanism* — everything you can do to a session or agent.
peregrin owns *policy* — who may act, when, from where (lock registry,
takeover decisions, multi-node merge/route/queue) plus orchestration
(campaigns, drivers, transport). `peregrin-node` drives sepia's
session plane over the wire — it never reimplements it.

## Layout

The crate map is generated from the workspace Cargo.tomls — see [AGENTS.md](AGENTS.md) for the dev map and [the docs site](https://9aia.com/peregrin/) for depth.

`libraries/` holds the workflow stdlib (reusable `.node.yml`,
`.snippet.yml`, `.field.yml` artifacts) plus the `peregrin` product
pack — the `peregrin` command ships as a tamias `bins[]` shim →
`tamias-driver-peregrin cli` engine passthrough (no hand-rolled
binary), with `peregrin-node`/`peregrin-hub` as payload bins.
Install: `tamias install 9aia:peregrin@0.2.0` (org registry — needs
`TAMIAS_REGISTRY_TOKEN`), or `cargo xtask pack` +
`tamias install --allow-drivers ./libraries/peregrin` from a checkout.
`project/` carries the ADRs — the engine's design history and spec.

## Develop

```bash
cargo xtask check    # fmt + clippy -D warnings
cargo xtask test     # whole suite (nextest if present)
cargo xtask pack     # stage libraries/peregrin payload (release bins)
cargo xtask coverage # llvm-cov report
cargo xtask install  # release-build peregrin-hub + tamias-driver-peregrin → ~/.local
```

[CONTRIBUTING](CONTRIBUTING.md) · [SECURITY](SECURITY.md) ·
[LICENSE](LICENSE) (MIT) · [project](project/index.md)

## Distribution (tamias)

Content distribution, provisioning, and service lifecycle live in
[**tamias**](https://github.com/9aia/tamias) (the host metaservice).
peregrin depends on it and ships a `peregrin provision` provider; use
`tamias install/deploy/uninstall/…` to move content.
