# Peregrin

Event-driven orchestration for agent workflows — campaigns, node
daemons, hub, remote transport. Rust workspace, ports-and-adapters.
Session *mechanism* lives in
[sepia](https://github.com/9aia/sepia) (sibling checkout at `../sepia`,
workspace path deps); peregrin owns policy + orchestration.

> The web UI (`peregrin-web`/`peregrin-web-core` + the Leptos SSR host)
> is experimental and lives on the `experimental/web` branch — `main`
> carries the headless hub only.

## Layout

- `peregrin-core` pure domain (events, campaigns, nodes) — no I/O.
- Ports: `peregrin-{driver,loader,store,bus,remote,config,secret,service,template}`;
  impls `peregrin-{port}-{impl}` (`-shell`, `-yaml`, `-toml`, `-keyring`,
  `-dotenv`, `-systemd`, `-http`).
- `peregrin-control` — arbitration (lock registry, takeover policy,
  merge/route) over the sepia session plane.
- `peregrin-node` (daemon: sepia plane + engine + api) · `peregrin-hub`
  (node-API gateway: sync engine + HTTP/SSE bridges + push) ·
  `peregrin-http` (REST+SSE) · `peregrin-node-api` (node port + wire
  DTOs) · `peregrin-sync`/`peregrin-outbox`/`peregrin-push` ·
  `peregrin-cli` (`peregrin` binary) · `peregrin-telemetry` ·
  `peregrin-testkit` · `xtask`.
- `libraries/` — workflow stdlib artifacts. `project/` — nested repo
  with the ADRs (the design history + port spec).

## Commands

```bash
cargo xtask check          # fmt + clippy -D warnings
cargo xtask test           # suite (nextest if present)
cargo xtask install        # release peregrin + hub → ~/.local
cargo xtask coverage       # llvm-cov report (HTML → target/llvm-cov/html);
                           # extra args pass through, e.g. --fail-under-lines 100
```

Git hooks in `.hooks/` (`core.hooksPath` = `.hooks`).

## Boundary with sepia

Mechanism vs policy: sepia does things *to* sessions (attach, prompt,
restore, drivers, ACP); peregrin decides who/when/where (locks,
takeover, merge, route, queue) and orchestrates (campaigns, drivers,
transport). `peregrin-node` consumes the session plane over the wire —
never link session internals into orchestration code.

## Rules

- Workspace lints strict (pedantic, `unwrap_used` warn, `unsafe_code`
  forbid); tests `#![allow]` liberally.
- Golden fixtures regenerate deliberately.
- `.config/nextest.toml` is the test-runner config.

## Docs

Diátaxis: `docs/tutorials/` `docs/how-to/` `docs/reference/`
`docs/explanation/`. `project/adrs/` is the engine design history —
read it before changing engine semantics.

<!-- myco:agent-awareness -->

## Agent awareness

- Session start / any work discussion: run `myco status --prompt`
  before proposing next steps — know the board first.
- User describes new work → offer to capture it (`myco todo add`,
  `issue new`, `proposal new`). Don't wait to be asked.
- Picking up a task: `myco resume <ID>` is the briefing.
- Wrapping up: `myco handoff <ID>` anything resumable; keep statuses
  honest (claimed → in-progress → blocked/done).

<!-- /myco:agent-awareness -->

<!-- sturn:agent-awareness -->

## Sturn awareness

- **No PRs.** The pushed branch/commit is the deliverable; review
  streams continuously (sturn WAL), merge is a human/sturn call.
- sturn-wired repo (`sturn.toml` present): `sturn claim "<glob>"
  --ws <ws>` *before* edits — claims surface conflicts in ms.
- Ops via `sturn write|rename|delete`; `sturn snap` folds the ws
  into a real commit; `sturn push` publishes heads + refs/sturn/*.
- `sturn leave` on exit — drops the actor's claims.

<!-- /sturn:agent-awareness -->
