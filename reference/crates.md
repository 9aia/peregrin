# Crates

Rust workspace, ports-and-adapters — ports are traits, impls are separate crates; the composition roots are `peregrin-node`, `peregrin-hub`, and the `peregrin` CLI.

<!-- crate-map -->
| crate | layer | purpose |
|---|---|---|
| `peregrin-core` | domain | Peregrin domain — events, campaign/node/graph model (pure) |
| `peregrin-template` | domain | Template expansion for campaign params |
| `peregrin-bus` | port | Event-bus port — campaign events to subscribers |
| `peregrin-config` | port | Config-source port |
| `peregrin-driver` | port | Campaign-driver port — node drivers run actions for the engine |
| `peregrin-loader` | port | Campaign-loader port — sources produce campaign definitions |
| `peregrin-remote` | port | Node-hub transport port |
| `peregrin-store` | port | Run/event state persistence port |
| `peregrin-config-toml` | adapter | TOML impl of the config port |
| `peregrin-driver-agent` | adapter | Agent impl of the campaign driver port |
| `peregrin-driver-council` | adapter | Council impl of the campaign driver port — one node as a voting round (ADR-0004) |
| `peregrin-driver-git` | adapter | Detached worktree-at-sha impl of the campaign driver port — runs an inner driver inside a materialized worktree |
| `peregrin-driver-github-webhook` | adapter | GitHub webhook ingress driver — HMAC-verified push events into the engine's ingress channel |
| `peregrin-driver-http` | adapter | HTTP impl of the campaign driver port |
| `peregrin-driver-human` | adapter | Human-in-the-loop impl of the campaign driver port |
| `peregrin-driver-ops` | adapter | Ops impl of the campaign driver port — proxies bounded tamias-driver queries over `tamias serve` |
| `peregrin-driver-schedule` | adapter | Temporal scheduling drivers — `delay` (fixed wait) and `schedule` (cron fire) over the engine's deferred-emission channel (ADR-0018) |
| `peregrin-driver-shell` | adapter | Shell impl of the campaign driver port |
| `peregrin-driver-sturn` | adapter | sturn WAL-ref poller — ingress impl of the campaign driver port (`sturn snap` → run) |
| `peregrin-driver-subworkflow` | adapter | `subworkflow` driver — run another campaign YAML as a nested, namespaced run with inputs/outputs mapping (ADR-0022) |
| `peregrin-http` | adapter | Peregrin HTTP API — axum driving adapter the peregrin-node daemon mounts (port of apps/server) |
| `peregrin-loader-yaml` | adapter | YAML impl of the campaign loader port |
| `peregrin-queue-sqlite` | adapter | SQLite QueueStore impl — the durable supersede queue behind campaign run --queue-store |
| `peregrin-remote-http` | adapter | HTTP+SSE impl of the transport port |
| `peregrin-sink-github-status` | adapter | GitHub commit-status sink — egress driver posting ci/&lt;campaign&gt; verdicts |
| `peregrin-store-json` | adapter | Durable JSONL impl of the event-store port (v1 ac-store-json) |
| `peregrin-telemetry` | adapter | OTEL-backed Observer impls + --tracing/--metrics provider wiring (ADR-0040) |
| `peregrin-auth` | service | Pairing + node credentials |
| `peregrin-git` | service | Explicit detached-worktree lifecycle — create, keep-for-debug, destroy, list, cleanup |
| `peregrin-queue` | service | Campaign-level run scheduler — supersede key, latest-wins/fifo dedupe, active-run cancel |
| `peregrin-session` | service | Session-plane boundary — spawns `sepia serve` and drives it over ndjson JSON-RPC |
| `peregrin-cli` | engine | The peregrin CLI — node ops over /api/*, sessions/projects/push verbs, pairing, service management, and serve |
| `peregrin-engine` | engine | Campaign engine — walks the event-driven node graph |
| `peregrin-hub` | engine | Node-API gateway — multi-node sync engine, HTTP/SSE bridges, push, auth (hypha-hub backend + peregrin kind policies) |
| `peregrin-node` | engine | The headless node daemon — composes driver discovery, the control plane, the meta overlay, and the API |
| `peregrin-testkit` | testkit | Peregrin conformance suites + fixtures |
| `xtask` | tooling | — |
