---
name: peregrin
description: Drive the peregrin orchestration substrate — workflows, bundles, run logs — via the peregrin CLI (a tamias bins[] shim), the driver's generated cli.<verb> ops surface, or the library API. Trigger when running/validating campaigns, installing bundles, reading run logs, or editing orchestration code in this repo.
---

# peregrin — the orchestration substrate

Peregrin owns policy + orchestration: campaigns (workflows) are the
grammar, drivers execute, events route. Session *mechanism* is sepia;
durable PM state is myco; ephemeral coordination is sturn. Read
`docs/reference/` before changing semantics — the schema docs are
the contract.

## The model in one breath

- **One grammar**: a campaign is a YAML `name:`/`config:`/`nodes:`
  graph; node config files are the same grammar (ADR-0080). Nodes
  subscribe (`on:` glob), fire a driver, emit (`emits:` types).
- **Planes**: ingress drivers emit events in (triggers), egress
  drivers consume terminal events (sinks). "Trigger"/"sink" are docs
  vocabulary — one unified `driver` registry.
- **Bundles** are distributable payload dirs (manifest +
  workflows/fragments), installed into tamias's store
  (`~/.local/share/tamias`) through `tamias install <ref>` —
  `./path`, `git:<url>[//subdir][#rev]`, `https://….tgz#sha256:…`.
  `libraries/` is the in-repo stdlib. Bundle lifecycle (install,
  deploy, service units, hooks) is **tamias**'s job — peregrin only
  ships the `tamias-driver-peregrin` provider.
- **Events** are a closed five-field envelope (`type`, `payload`,
  `source`, `id`, `ts`) — `docs/reference/event-envelope.md`. Run
  logs are JSONL via `peregrin-store-json`.

See `references/substrate.md` for the fuller picture.

## The verb surface

The `peregrin` command is a tamias `bins[]` shim (T-012) — a
generated script → `tamias bin exec` → `tamias-driver-peregrin cli`
engine passthrough; argv is identical to the retired binary. Install:
`cargo xtask pack` + `tamias install --allow-drivers
./libraries/peregrin`.

Human CLI (`peregrin`) vs agent surface — `tamias-driver-peregrin`'s
`serve` JSON-RPC answers `cli.<verb>` ops derived from the engine's
own clap tree (the standalone `peregrin-mcp` server and the
driver's embedded `mcp` feature are gone — MCP, where wanted,
comes from pavo's generic `tamias-driver-mcp peregrin`):

| Intent | CLI / tamias | `cli.*` op |
| --- | --- | --- |
| Run a campaign | `peregrin campaign run <wf>` *(surface)* | `cli.campaign.submit` + `cli.campaign.status`/`events` (submit+poll — never blocks) |
| Check a workflow | `peregrin campaign validate <wf> [--bundle b]` | `cli.campaign.validate` |
| Install a bundle | `peregrin bundle install <ref>` ≡ `tamias install` | `cli.bundle.install` |
| Drop a bundle | `peregrin bundle remove <name>` | `cli.bundle.remove` |
| List bundles | `peregrin bundle list` | `cli.bundle.list` |
| Bundle detail | `peregrin bundle info <name>` | `cli.bundle.info` |
| Scan run logs | `peregrin logs runs [--dir d]` | `cli.logs.runs` |
| Tail a run log | `peregrin logs events <path> [--type t] [--limit n]` | `cli.logs.events` (bounded) |
| Explain a run log | `peregrin logs explain <path>` | `cli.logs.explain` |
| Node ops | `peregrin sessions|projects|config|push …` | `cli.sessions.*`, `cli.projects.*`, … |
| Push-run history | `peregrin runs|log|explain --history <log>` | `cli.runs`, `cli.log`, `cli.explain` |

Bundle ops are thin delegates over the **tamias** install pipeline
and store (`$TAMIAS_HOME/store`, `$TAMIAS_CACHE`) — `bundle install`
lands where `tamias install` does. `campaign submit` writes each
run's JSONL log to `<peregrin-home>/runs/<run-id>.jsonl`; poll
`campaign status` for the verdict, `campaign events`/`logs events`
for the bounded tail. The old MCP `campaign_run` (block-until-done)
is now submit+poll — loop `status` until `verdict` is `done`/
`failed`/`cancelled`.

Ops mechanics, params, `mutates`, and the `surfaces` list:
`references/ops-surface.md`; full contract:
`docs/reference/ops.md`. Everything is advisory — there is no
enforcement layer here; coordination lives in sturn.

## Coordinating with sturn

- **No PRs.** The pushed branch/commit is the deliverable; merge is a
  human/sturn call.
- sturn-wired repo (`sturn.toml` present): `sturn claim "<glob>" --ws
  <ws>` *before* editing; `sturn snap` folds the ws into a commit;
  `sturn push` publishes; `sturn leave` on exit.
- File claims are the conflict-early mechanism — claim the crates and
  doc paths you'll touch, not the world.

## House rules

- `cargo xtask check` (fmt + clippy `-D warnings`) and
  `cargo xtask test` before pushing; `cargo xtask coverage` measures —
  new code ships at 100% line coverage.
- Workspace lints are pedantic + `unwrap_used`/`expect_used` warn +
  `unsafe_code` forbid; tests `#![allow]` liberally.
- Docs follow Diátaxis — `docs/{tutorials,how-to,reference,
  explanation}/`; engine semantics history lives in `project/adrs/`
  (read it before touching the engine).
- PM state is myco (`myco status`, `myco todo`, `myco handoff`) —
  never TODOs/status in READMEs or code comments.

Full conventions: `references/conventions.md`.
