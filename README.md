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
- **Hub** — `peregrin-hub` aggregates nodes into one view and serves
  the app (`peregrin-web`, Leptos) — sessions today, campaigns next.
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

| Crate | Role |
| --- | --- |
| `crates/peregrin-core` | events, campaign/node/graph model — pure domain |
| `crates/peregrin-engine` | graph walker/scheduler |
| `crates/peregrin-driver` + `-shell` | campaign driver port + impls |
| `crates/peregrin-loader` + `-yaml` | campaign loading |
| `crates/peregrin-{template,store,bus,remote(-http)}` | engine ports |
| `crates/peregrin-{config(-toml),secret,keyring,dotenv,service,systemd}` | platform ports (sturn/bupha pattern) |
| `crates/peregrin-control` | arbitration — lock registry, takeover policy, merge/route |
| `crates/peregrin-{node,hub,http,sync,outbox,push}` | daemons + transport |
| `crates/peregrin-{web,web-core}` | the Leptos app |
| `crates/peregrin-{cli,auth,acp-client,telemetry,testkit}` | binaries, credentials, sepia-wire client, OTEL, fixtures |

`libraries/` holds the workflow stdlib (reusable `.node.yml`,
`.snippet.yml`, `.field.yml` artifacts). `project/` carries the ADRs —
the engine's design history and spec.

## Develop

```bash
cargo xtask check    # fmt + clippy -D warnings + web markup gate
cargo xtask test     # whole suite (nextest if present)
cargo xtask install  # release-build peregrin + hub + site → ~/.local
cargo xtask site     # wasm bundle + PWA assets → target/site
cargo xtask smoke    # headless-Chrome console probe
cargo xtask e2e      # full node+hub+browser suite
```

[CONTRIBUTING](CONTRIBUTING.md) · [SECURITY](SECURITY.md) ·
[LICENSE](LICENSE) (MIT) · [project](project/index.md)
