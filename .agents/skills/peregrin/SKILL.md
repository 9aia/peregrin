---
name: peregrin
description: Drive the peregrin orchestration substrate — workflows, bundles, run logs — via the peregrin CLI, the peregrin-mcp server, or the library API. Trigger when running/validating campaigns, installing bundles, reading run logs, or editing orchestration code in this repo.
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
  workflows/fragments), installed into `<peregrin-home>/store`
  through ADR-0079 refs (`./path`, `git:<url>[//subdir][#rev]`,
  `https://….tgz#sha256:…`). `libraries/` is the in-repo stdlib.
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

Human CLI (`peregrin`) vs agent surface (`peregrin-mcp`, stdio
JSON-RPC — the same library calls, no shelling out):

| Intent | CLI | MCP tool |
| --- | --- | --- |
| Run a campaign | `peregrin campaign run <wf>` | `campaign_run` |
| Check a workflow | `peregrin campaign validate <wf>` | `validate` |
| Install a bundle | `peregrin install <ref>` | `bundle_install` |
| Drop a bundle | `peregrin remove <name>` | `bundle_remove` |
| List bundles | `peregrin list` | `bundle_list` |
| Bundle detail | `peregrin info <name>` | `bundle_info` |
| Find runs | *(logs on disk)* | `runs` |
| Tail a run | — | `events` |
| Summarize a run | — | `explain` |

Tool schemas + params: `references/mcp-tools.md`. Everything is
advisory — there is no enforcement layer here; coordination lives
in sturn.

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
