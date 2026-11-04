# House rules — working in this repo

## Flow

- **No PRs.** The pushed branch/commit is the deliverable; review
  streams continuously (sturn WAL), merge is a human/sturn call.
- sturn-wired repo (`sturn.toml` present): `sturn claim "<glob>"
  --ws <ws>` before edits; `sturn write|rename|delete` for ops;
  `sturn snap` folds the ws into a commit; `sturn push` publishes
  heads + `refs/sturn/*`; `sturn leave` on exit drops claims.
- Commit style: `feat:`/`fix:`/`style:`/`docs:` + a body when the
  why isn't obvious. Golden fixtures regenerate deliberately.

## Gates

```bash
cargo xtask check     # fmt + clippy -D warnings + web markup gate
cargo xtask test      # suite (nextest if present)
cargo xtask coverage  # llvm-cov; --fail-under-lines 100 for new code
```

- Workspace lints: pedantic + `unwrap_used`/`expect_used` warn,
  `unsafe_code` forbid. Tests `#![allow(clippy::unwrap_used, …)]`
  liberally.
- `cfg(coverage)` gates paths that can't run in-process (blocking
  stdio loops, service lifecycle, spawned binaries) — see
  `peregrin-cli`'s service backends and `peregrin-mcp::serve`.
- `peregrin-web`: `#[cfg]` gates behavior, never markup shape —
  `xtask check` enforces.

## Docs

Diátaxis under `docs/`: `tutorials/` (learning), `how-to/` (tasks),
`reference/` (contracts — schema docs are P0), `explanation/`
(why). Engine design history is `project/adrs/` — read the relevant
ADR *before* changing engine semantics; new decisions get a new ADR
(the `write-adr` skill covers the shape).

## PM

All project state is myco — `myco status --prompt` at session start,
`myco todo add` for new work, `myco resume <ID>` to pick up,
`myco handoff <ID>` at wrap-up. Never track tasks/status in READMEs,
docs, or code comments.
