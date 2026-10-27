# Peregrin

Event-driven orchestration for agent workflows — campaigns, node
daemons, hub, remote transport — plus the session-management app.
Rust workspace, ports-and-adapters. Session *mechanism* lives in
[sepia](https://github.com/9aia/sepia) (sibling checkout at `../sepia`,
workspace path deps); peregrin owns policy + orchestration.

## Layout

- `peregrin-core` pure domain (events, campaigns, nodes) — no I/O.
- Ports: `peregrin-{driver,loader,store,bus,remote,config,secret,service,template}`;
  impls `peregrin-{port}-{impl}` (`-shell`, `-yaml`, `-toml`, `-keyring`,
  `-dotenv`, `-systemd`, `-http`).
- `peregrin-control` — arbitration (lock registry, takeover policy,
  merge/route) over the sepia session plane.
- `peregrin-node` (daemon: sepia plane + engine + api) · `peregrin-hub`
  (SSR host) · `peregrin-http` (REST+SSE) · `peregrin-sync`/
  `peregrin-outbox`/`peregrin-push` · `peregrin-web`/`peregrin-web-core`
  (Leptos app) · `peregrin-cli` (`peregrin` binary) ·
  `peregrin-telemetry` · `peregrin-testkit` · `xtask`.
- `libraries/` — workflow stdlib artifacts. `project/` — nested repo
  with the engine ADRs (the port spec). `adrs/` — top-level ADRs.

## Commands

```bash
cargo xtask check          # fmt + clippy -D warnings + web markup gate
cargo xtask test           # suite (nextest if present)
cargo xtask install        # release peregrin + hub + site → ~/.local
cargo xtask site           # wasm bundle + PWA assets → target/site
cargo xtask dev            # cargo-leptos watch: rebuild wasm+hub on save,
                           # auto-refresh — dev hub on :13002 (prod :13000)
cargo xtask smoke          # headless-Chrome console probe (post-UI-change)
cargo xtask e2e            # node+hub+browser suite (needs site first)
cargo xtask coverage       # llvm-cov report (HTML → target/llvm-cov/html);
                           # extra args pass through, e.g. --fail-under-lines 100
```

UI stack: Tailwind v4 + typography — `bun install && bun run css`
writes `crates/peregrin-web/style/main.css` (generated — edit
`input.css`, never `main.css`). Git hooks in `.hooks/`
(`core.hooksPath` = `.hooks`).

## Boundary with sepia

Mechanism vs policy: sepia does things *to* sessions (attach, prompt,
restore, drivers, ACP); peregrin decides who/when/where (locks,
takeover, merge, route, queue) and orchestrates (campaigns, drivers,
transport). `peregrin-node` consumes the session plane over the wire —
never link session internals into orchestration code.

## Rules

- Workspace lints strict (pedantic, `unwrap_used` warn, `unsafe_code`
  forbid); tests `#![allow]` liberally.
- `peregrin-web`: `#[cfg]` gates behavior, never markup shape
  (`view!`/`impl IntoView`) — enforced by `xtask check`.
- Golden fixtures regenerate deliberately.
- `.config/nextest.toml` is the test-runner config.

## Docs

Diátaxis: `docs/tutorials/` `docs/how-to/` `docs/reference/`
`docs/explanation/`. `project/adrs/` is the engine design history —
read it before changing engine semantics.
