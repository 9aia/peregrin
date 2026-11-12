# Contributing

Issues and PRs are **disabled** — the project is agent-maintained.
Fork freely under MIT.

## Layout

`crates/` — rust workspace, ports-and-adapters: `peregrin-core` is
pure domain; ports are `peregrin-{driver,store,bus,remote,config,
loader,template}`; impls are `peregrin-{port}-{impl}`
crates; `peregrin-node`/`peregrin-hub` are the daemons;
`peregrin-cli` is the engine lib + `tamias-driver-peregrin` (the
`peregrin` command itself is the tamias pack's `bins[]` shim in
`libraries/peregrin` — there is no hand-rolled binary). The substrate
crates — `hypha-{node-api,outbox,control,sync,push,http,hub}` plus
the earlier `hypha-{secret,keyring,dotenv,service,systemd,
telemetry}` leaves — live in [9aia/hypha](https://github.com/9aia/hypha)
and arrive here as git deps; `peregrin-http` composes the generic
`hypha-http` routes with the session surface under one listener.
`libraries/` is the workflow stdlib; `project/` the ADRs.

## Boundary

Policy + orchestration live here; session *mechanism* lives in
[sepia](https://github.com/9aia/sepia). If a feature is "something you
do to a session" it belongs to sepia's wire; if it's "who may act,
when, where" or campaign execution, it belongs here.

## Rules of thumb

- New driver/transport/store = a new `peregrin-{port}-{impl}` crate,
  not a core change.
- `cargo xtask check` and `cargo xtask test` must stay green.
- Coverage: `cargo xtask coverage` — cargo-llvm-cov HTML report in
  `target/llvm-cov/html`; pass `--fail-under-lines 100` to gate.
- Commits: conventional prefixes (`feat:`/`fix:`/`refactor:`/`docs:`/`chore:`).
- Never commit secrets — `hypha-secret` takes names, not values.

## Docs

Follow Diátaxis: `docs/tutorials/` (learn) · `docs/how-to/` (tasks) ·
`docs/reference/` (lookup) · `docs/explanation/` (why).

## Releasing

- Org policy (project ADR-0004): **crates.io is tamias-only** —
  peregrin does not publish there. A release is a `<unit>-v<ver>`
  tag plus a manual `tamias publish libraries/peregrin` of the pack.
  The release plane is **tarsi** (`tarsi-serve` ops — project
  ADR-0005); peregrin's own `libraries/release` bundle drives those
  ops in campaigns, and the per-repo `scripts/release.sh` is being
  converted to a thin shim over `tarsi-serve` (PL-014 E1).
- The current `scripts/release.sh` still carries the legacy
  per-crate crates.io publish (`peregrin-cli` et al., dep-ordered)
  — it predates the shim conversion. `peregrin-testkit` is
  `publish = false`; the `peregrin` command is the tamias pack's
  `bins[]` shim, and the `peregrin-node`/`peregrin-hub` payload bins
  ship via `libraries/peregrin` (`cargo xtask pack` stages them).
- Release binaries: `scripts/release.sh` can still build
  `peregrin-<target>.tar.gz` (linux x86_64, macOS aarch64/x86_64)
  carrying the payload bins (`peregrin-node`, `peregrin-hub`,
  `tamias-driver-peregrin`) via `dist`/`gh` —
  `peregrin-x86_64-unknown-linux-gnu.tar.gz` is the name installers
  probe once the first release exists.
