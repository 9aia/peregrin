# Peregrin

Generic distributed orchestration — campaigns of event-driven
workflows over generic *entities* (`{id,kind,indexDoc}` + namespaced
ops + snapshots) across paired nodes. Rust workspace,
ports-and-adapters. Sessions register as `kind="sepia-session"`
entities via the [sepia](https://github.com/9aia/sepia) integration
(sibling checkout at `../sepia`, workspace path deps); peregrin owns
the entity plane — projection, outbox, locks, ops, snapshots — and
never names a sepia type.

> The web UI (`peregrin-web`/`peregrin-web-core` + the Leptos SSR host)
> is experimental and lives on the `experimental/web` branch — `main`
> carries the headless hub only.

## Layout

- `peregrin-core` pure domain (events, campaigns, nodes) — no I/O.
- Ports: `peregrin-{driver,loader,store,bus,remote,config,secret,service,template}`;
  impls `peregrin-{port}-{impl}` (`-shell`, `-yaml`, `-toml`, `-keyring`,
  `-dotenv`, `-systemd`, `-http`).
- `peregrin-control` — entity arbitration (lock registry, takeover
  policy, merge/route) — generic `EntityId`/`kind` mechanics.
- `peregrin-node` (daemon: session plane + engine + api) · `peregrin-hub`
  (entity gateway: merged index + ops outbox + HTTP/SSE bridges + push) ·
  `peregrin-http` (REST+SSE — `/api/entities*` substrate surface,
  `/api/sessions*` integration surface) · `peregrin-node-api` (`EntityApi`
  trait + entity DTOs + `HttpEntityApi`/`FakeEntityApi`) ·
  `peregrin-sync` (entity projection + sync engine) /
  `peregrin-outbox` (entity-scoped op queue) / `peregrin-push` ·
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

Substrate vs integration: peregrin runs entities (project, route,
queue, lock, snapshot, transfer) — never names a sepia type. The
session plane is one entity kind (`sepia-session`) hosted by
`peregrin-node`: `/api/entities*` is the substrate surface,
`/api/sessions*` the integration's. Op kinds are namespaced
(`sepia.prompt`, `sepia.attach`, `entity.delete`); snapshots are
kind-shaped bytes moved opaquely. `project/adrs/0091` is the contract.

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
