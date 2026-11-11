# Peregrin

Generic distributed orchestration — campaigns of event-driven
workflows over generic *entities* (`{id,kind,indexDoc}` + namespaced
ops + snapshots) across paired nodes. Rust workspace,
ports-and-adapters. Sessions register as `kind="sepia-session"`
entities via the [sepia](https://github.com/9aia/sepia) integration
(sibling checkout at `../sepia`, workspace path deps). The generic
substrate — entity plane, projection, outbox, locks, ops, snapshots,
the HTTP surface — lives in [9aia/hypha](https://github.com/9aia/hypha)
and is consumed here as git deps; peregrin keeps the integration
side (session/campaign planes, `PEREGRIN_*` env contract) and never
names a sepia type in the substrate.

> The web UI (`peregrin-web`/`peregrin-web-core` + the Leptos SSR host)
> is experimental and lives on the `experimental/web` branch — `main`
> carries the headless hub only.

## Layout

- `peregrin-core` pure domain (events, campaigns, nodes) — no I/O.
- Ports: `peregrin-{driver,loader,store,bus,remote,config,template}`;
  impls `peregrin-{port}-{impl}` (`-shell`, `-yaml`, `-toml`, `-http`).
  The substrate crates — `hypha-{node-api,outbox,control,sync,push,
  http,hub}` plus the earlier `hypha-{secret,keyring,dotenv,service,
  systemd,telemetry}` leaves — are extracted to
  [9aia/hypha](https://github.com/9aia/hypha) and consumed as git
  deps. `peregrin-http` composes them: `compose`/`hypha_state` merge
  `hypha_http`'s generic routes (`/api/entities*`, `/api/events`,
  `/api/node`, `/api/pair`, `/api/auth/*`, `/api/push/*`, `/api/fs`,
  `/api/user`, `/api/health`) with the session routes
  (`/api/sessions*`, agents, projects, config, client keypair) under
  one listener, unifying feed/pairing/push/kind-registry behind the
  scenes.
- `peregrin-node` (daemon: session plane + campaign engine +
  composition root) · `peregrin-hub` (thin `hypha-hub` gateway —
  `SepiaSessionPolicy`/`CampaignKindPolicy` + the `PEREGRIN_*` env
  mapping) · `peregrin-http` (session routes + composition, above) ·
  `peregrin-cli` (`peregrin` binary + the `tamias-driver-peregrin`
  dual-surface driver — pavo `manifest`/`serve`/`ops`/`run` plus the
  tamias `install`/`deploy`/`uninstall`/`hooks`/`health`/`status`/
  `logs`/`config-schema` verbs; the CLI is lib-ified as
  `peregrin_cli::cli` — `command()`/`parse_from()`/`run_from()` —
  and `engine_ops` derives the `cli.<verb>` ops manifest from that
  clap tree. Unbounded verbs (`serve`, `campaign serve`/`run`,
  `sessions attach`/`stream`, `events`) are `surfaces`, never ops;
  remote verbs are ops taking `--node`/`--token` and degrade
  cleanly with no node. Features `cli`/`mcp` embed the
  pavo-driver-kit generated surfaces) · `hypha-telemetry` (git dep) ·
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

Substrate vs integration: the substrate (entities, projection,
outbox, locks, sync, the generic HTTP surface) is hypha's; peregrin
runs the integration — sessions register as `kind="sepia-session"`
entities and campaigns as `kind="campaign"`, both hosted by
`peregrin-node`. `/api/entities*` + the generic routes are the
substrate surface (`hypha-http`), `/api/sessions*` the integration's;
`peregrin-http` composes them into one router. Op kinds are
namespaced (`sepia.prompt`, `campaign.*`, `entity.delete`); snapshots
are kind-shaped bytes moved opaquely. `project/adrs/0091` is the
contract. Op kinds are namespaced
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

## Distribution & host lifecycle live in tamias

Bundle **distribution**, **provisioning**, **deploy**, and **service**
lifecycle are **tamias**'s job — the host/shell metaservice (sibling repo
`9aia/tamias`, consumed here via a rev-pinned git dep). peregrin ships the
**provider** tamias shells:

```
peregrin provision {install,deploy,uninstall} --dir <dir> --name <name> [--config <f>]
```

`peregrin install/remove/…`, `create`, `deploy`, `destroy`, and `service`
verbs are **gone** from the peregrin CLI — run them via `tamias`. Host
diagnostics (`doctor`) also live in tamias now.
