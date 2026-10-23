# Contributing

Issues and PRs are **disabled** — the project is agent-maintained.
Fork freely under MIT.

## Layout

`crates/` — rust workspace, ports-and-adapters: `peregrin-core` is
pure domain; ports are `peregrin-{driver,store,bus,remote,config,
secret,service,loader,template}`; impls are `peregrin-{port}-{impl}`
crates; `peregrin-node`/`peregrin-hub` are the daemons;
`peregrin-cli` (`peregrin`) is the composition root.
`libraries/` is the workflow stdlib; `project/` the ADRs.

## Boundary

Policy + orchestration live here; session *mechanism* lives in
[sepia](https://github.com/9aia/sepia). If a feature is "something you
do to a session" it belongs to sepia's wire; if it's "who may act,
when, where" or campaign execution, it belongs here.

## Rules of thumb

- New driver/transport/store = a new `peregrin-{port}-{impl}` crate,
  not a core change.
- `cargo xtask check` and `cargo xtask test` must stay green; the web
  UI also gates `cargo xtask smoke`.
- Coverage: `cargo xtask coverage` — cargo-llvm-cov HTML report in
  `target/llvm-cov/html`; pass `--fail-under-lines 100` to gate.
- Commits: conventional prefixes (`feat:`/`fix:`/`refactor:`/`docs:`/`chore:`).
- Never commit secrets — `peregrin-secret` takes names, not values.

## Docs

Follow Diátaxis: `docs/tutorials/` (learn) · `docs/how-to/` (tasks) ·
`docs/reference/` (lookup) · `docs/explanation/` (why).
