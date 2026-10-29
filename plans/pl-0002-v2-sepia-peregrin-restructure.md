+++
id = "PL-002"
title = "v2 — sepia+peregrin restructure"
status = "open"
created = "2026-10-29"
priority = "medium"
+++

# v2 — Sepia + Peregrin restructure plan

Working document. Status: **merged to `main` on both repos**
(2026-10-19) — sepia's `master` was renamed to `main`; `v2` branches
kept on the remotes. Peregrin suite: **346/346 green**
(`cargo xtask test`); sepia 368 green. Docs live at
`9aia.com/sepia/` + `9aia.com/peregrin/` (orphan `docs` branches →
VitePress → GitHub Pages).

### Sepia v2 — done, green

- `v2` off `rust-rewrite`, commit `refactor: v2 — the session
  abstraction, standalone`. `cargo xtask check` + 368 tests pass.
- `sepia-driver-sdk→sepia-driver`, `sepia-driver-host→sepia-host`,
  `sepia-control→sepia-session` (all mechanism — `merged.rs` is the
  local store-union, it stayed). Node composition moved to
  `sepia-session::local` (`build`/`Paths`/`base_options`).
- App crates removed (node/hub/web/web-core/http/sync/outbox/push —
  imported by peregrin). `sepia` CLI = store/driver/config verbs +
  **`sepia serve`**: the session plane as ndjson JSON-RPC on stdio
  (`crates/sepia-cli/src/serve.rs`, protocol v1, real dispatch incl.
  attach/prompt/permission/restore/rewind/subscribe).
- CONTRIBUTING/SECURITY/LICENSE/project/(TODO·BACKLOG·adrs)/mimus.toml
  added; docs trimmed to domain refs; ci/release updated.

### Peregrin v2 — landed, compiles

- `v2` off `main`, commits `refactor: v2 — the orchestration platform,
  in Rust` + test fix. `cargo check --workspace` green.
- TS tree removed (on `main` as spec); app crates imported +
  renamed; 20 new platform skeleton crates; `peregrin` (cli+serve) and
  `peregrin-hub` binaries; `peregrin-node` composes
  `sepia-session::local` (linked — subprocess cutover is follow-up).
- sepia deps are **path deps to `../sepia`** — swap to git deps once
  pushed.

### Done since

- **SessionPlane port landed**: `peregrin-session` has the full typed
  wire surface + `SessionPlane` trait with `LocalPlane` (in-process)
  and `ServicePlane` (`sepia serve` subprocess) impls. `peregrin-http`
  `AppState.plane` is `Arc<dyn SessionPlane>`; `peregrin-node` picks
  via `PEREGRIN_SESSION_PLANE` (default in-process — convert/import
  seams still need direct store access; `service` is opt-in).
- **Arbitration wire begun**: hub `owner()` delegates its pick policy
  to `peregrin_control::pick_owner` (OwnershipRow + agent tiebreak).
- **Docs branches live**: orphan `docs` branch on both repos, worktree
  at `docs/`, VitePress + Diátaxis skeletons; sepia's domain mds moved
  to `docs/reference/`. Pushed.
- **Pushed + published metadata**: both `v2` + `docs` on GitHub;
  descriptions + topics set (bupha-style), repos stay public.
- **Stale pruned**: `.windsurfrules`, `rules/`, TS skills removed
  (kept write-adr + diataxis).
- Verified: 36/36 tests pass in the touched crates (session/http/
  node — including node e2e); clippy `-D warnings` clean workspace;
  `cargo xtask check` green through the pre-push hook on both pushes.
- **Deps**: sepia now comes via git deps (branch `v2`); local dev
  overrides via `[patch]` in `~/.cargo/config.toml` or
  `.cargo/config.toml.example`.
- **Pages**: GitHub Pages enabled on both repos (Actions source) —
  `9aia.com/sepia` + `9aia.com/peregrin`; deploy workflows are on the
  `docs` branches and trigger on push.

### Still open

- Full suite re-run to confirm remaining hub/sync/transfer e2e (the
  fixes that broke them — binary resolution — landed mid-session).
- Flip `PEREGRIN_SESSION_PLANE=service` to default once the wire
  covers store ops (convert/import currently in-process only).
- `peregrin-control` deeper wiring (LockRegistry/Arbitration decisions
  at hub attach/prompt call sites).
- Engine/loader/template impls (port TS campaign semantics per ADRs).
- Merge `v2` → main lines when you're satisfied; GitHub Pages deploy
  for the docs sites.

Once approved, this file can move into a repo's `project/` (or be split
into per-repo `project/plans/v2.md`) when the `v2` branches exist.

## North star

- **sepia** — the agent-session abstraction, in Rust. A library
  workspace plus a first-class `sepia` CLI. Core value: *understand and
  manage coding-agent sessions agnostically* — any agent, any store
  format, embeddable by anyone, no peregrin required.
- **peregrin** — the orchestration/automation platform *and* the app,
  in Rust. Event-driven campaigns, node daemons, hub, remote transport
  (p2p later), multi-actor arbitration — plus the session-management
  app that used to live in sepia. Peregrin *embeds* sepia.

## Boundary principle: mechanism vs policy

- **sepia = mechanism.** Everything you can do *to* a session or agent:
  probe liveness, attach (ro/rw), prompt, cancel, permission
  round-trips, watch update streams, restore/rewind execution, agent
  respawn, config/rules/skills inspection, format conversion.
- **peregrin = policy.** Arbitration across actors and machines: lock
  registry, takeover decisions (who preempts whom, queueing), merged
  multi-node views, routing to the owning node, offline write ordering.
  Takeover = peregrin decides → sepia executes (detach → respawn →
  re-attach).
- Corollary: standalone sepia needs no arbitration (one user, one
  machine — "live session held by pid N" is detection, not policy).

## Repo anatomy (both repos, copied from bupha)

- `crates/` flat workspace, explicit members, ports/adapters naming:
  `{repo}-{port}` for the trait crate, `{repo}-{port}-{impl}` or
  `{repo}-{impl}` for adapters.
- Workspace `Cargo.toml`: shared lints (pedantic, `unsafe_code` forbid),
  `dev` profile `line-tables-only`, `dist` profile (fat LTO, 1 CGU).
- `xtask` for check/test/install/site/smoke automation.
- `README.md` product story · `CONTRIBUTING.md` · `SECURITY.md` ·
  `LICENSE` (MIT) · `AGENTS.md` layout+rules file.
- `docs/` Diátaxis (`tutorials/ how-to/ reference/ explanation/`) +
  `gen-*.py` codegen for cli/config/crate-map reference, VitePress site.
- `project/` → `index.md`, `TODO.md`, `BACKLOG.md`, `adrs/`.
- `.agents/skills/` (diataxis writing skill + repo-specific skills).
- `rules/` + `mimus.toml` + `.kuda/` (dogfood the 9aia toolchain).
- `examples/` config/examples · `scripts/release.sh` ·
  `dist-workspace.toml` (cargo-dist) · `.github/` CI until self-hosted.
- Git hooks (`.hooks/` or equivalent): fmt on commit, `xtask check`
  on push.
- **Docs site**: dedicated `docs` branch per repo, checked out as a
  worktree at `docs/` (bupha's model) — VitePress source + generated
  rustdoc (`public/api/`) + `gen-*.py` codegen, deployed to the same
  docs host as today. The `docs` branches are independent of `v2`;
  create them when the v2 docs skeleton is ready.
- **Skills**: `skills-lock.json` + `.agents/skills/` installed via
  skills.sh — diataxis writing skill in both repos (same as today).
- **Quality bar**: 100% coverage *goal* in both repos — `xtask test`
  wraps nextest, `xtask coverage` wraps cargo-llvm-cov, `sepia-testkit`/
  `peregrin-testkit` conformance suites gate the wire contracts.

## sepia v2 — crate map

Reshaped *in place* on `v2` (off `rust-rewrite`) with `git mv`/`git rm`
so blame survives. Old app code remains in history and on
`rust-rewrite`/`master`.

| Crate | From | Role |
| --- | --- | --- |
| `sepia-core` | same | Session IR, `SessionRepository` port, restore/rewind planners — pure domain |
| `sepia-proto` | same | `SessionEvent` + wire types |
| `sepia-agents` | **new** (from `docs/agent-configs.md` + driver manifests) | agent catalog: identity, capabilities, config/rules/skills locations, ACP spawn spec |
| `sepia-acp` | same | tolerant ACP client over ndjson stdio |
| `sepia-driver` | `sepia-driver-sdk` | driver manifest + ndjson JSON-RPC serve loop |
| `sepia-host` | `sepia-driver-host` | driver discovery + spawn + respawn |
| `sepia-driver-{devin,cline,claude,cursor}` | same | store adapter binaries (discovered, not linked) |
| `sepia-session` | `sepia-control` minus `merged.rs` minus arbitration | session ops: liveness probe, attach modes, prompt/cancel/permissions, restore/rewind exec, respawn |
| `sepia-meta` | same | overlay store (titles/pins/projects/run spans) |
| `sepia-convert` | same | store format conversion |
| `sepia-telemetry` | same | opt-in OTEL layer (sepia-side only — peregrin gets its own, see open #9) |
| `sepia-testkit` | same | conformance suites + golden fixtures — the "agnostic" guarantee |
| `sepia-cli` | slimmed | `sepia` binary: sessions list/inspect/prompt/watch/attach/restore/convert, `driver list`, agent config inspect, **plus the stdio service mode peregrin consumes** |
| `xtask` | same | workspace automation |

Removed (moved to peregrin): `sepia-node`, `sepia-hub`, `sepia-web`,
`sepia-web-core`, `sepia-http`, `sepia-sync`, `sepia-outbox`,
`sepia-push`, `sepia-control` (multi-node + arbitration parts), cli
verbs `serve`/`pair`/`node`/`service`, hub credential store.

Docs kept (already domain refs): `session-formats.md`, `protocol.md`,
`agent-configs.md` — folded into the `docs`-branch VitePress `reference/`
section. DEPLOY.md leaves with the app (rewritten as peregrin deploy
docs). `transfer` (cross-node session copies): session-domain —
proposed home `sepia-session` or `sepia-transfer`. [OPEN]

Driver discovery paths must be **platform-agnostic** (`dirs`-style
data-dir resolution, not hardcoded `~/.local/share/sepia/drivers`) —
Linux-only today, win/mac support is a backlog item; the resolution
code must not assume XDG.

## peregrin v2 — crate map

`v2` off `main`. TS tree removed on the branch (it stays on `main` as
spec/reference). App crates imported from sepia's `rust-rewrite` tree
and renamed `sepia-* → peregrin-*`, `SEPIA_* → PEREGRIN_*`.

### Absorbed app (from sepia)

| Crate | From | Role |
| --- | --- | --- |
| `peregrin-node` | `sepia-node` | machine daemon — drives the `sepia` subprocess session plane **and** runs the campaign engine |
| `peregrin-hub` | `sepia-hub` | aggregation + app host (SSR) — ships as its own binary (see below) |
| `peregrin-web` / `peregrin-web-core` | `sepia-web` / `sepia-web-core` | the app — sessions UI now, campaigns UI later; `web-core` stays pure (no DOM/leptos) |
| `peregrin-http` | `sepia-http` | axum REST + SSE |
| `peregrin-control` | `sepia-control` (merged/arbitration) | lock registry, takeover policy, merged lists, routing, offline queue ordering |
| `peregrin-sync` | `sepia-sync` | node→hub projection |
| `peregrin-outbox` | `sepia-outbox` | durable write queue |
| `peregrin-push` | `sepia-push` | web push |
| `peregrin-auth` | hub credentials + `pair` | pairing, node-token credential store |

### New platform crates (port of the `ac-*`/`handl-*` concepts)

| Crate | Port / role |
| --- | --- |
| `peregrin-core` | events, campaign/node/graph model — pure domain |
| `peregrin-engine` | graph walker/scheduler (spec: `project/adrs`, TS `ac-engine`) |
| `peregrin-driver` + `peregrin-driver-{shell,http,agent,human,webhook,polling,stdin,...}` | campaign node drivers (from `ac-driver-*`) |
| `peregrin-loader` + `peregrin-loader-yaml` | campaign loading/include/snippet expansion |
| `peregrin-template` | template engine |
| `peregrin-store` + impls | run/event state persistence |
| `peregrin-bus` | event system port — local impl now |
| `peregrin-remote` + `peregrin-remote-http` | node↔hub transport port; HTTP+SSE impl now, p2p impl (`-iroh`/`-libp2p`) behind the port later [ADR needed] |
| `peregrin-config` + `peregrin-config-toml` | config source port |
| `peregrin-secret` + `peregrin-keyring` + `peregrin-dotenv` | secrets port — sturn pattern (`sturn-secret` port + keyring/dotenv adapters) |
| `peregrin-service` + `peregrin-systemd` | install/service port |
| `peregrin-cli` | `peregrin` binary — cli verbs + `serve` runs the node daemon (one install per machine) |
| `peregrin-hub` (bin) | `sepia-hub` | `peregrin-hub` binary — aggregation + serves the Leptos app; separate so daemon-only machines skip the web stack |
| `peregrin-testkit` | conformance/fixtures |
| `xtask` | workspace automation |

Kept from the TS repo: `libraries/` (the workflow stdlib — content,
not TS code) and `project/` (nested repo — the 30+ ADRs are the
engine's design history; keep as the spec source).

The story: `peregrin-node` = one daemon hosting two capability planes —
sessions (spawns the `sepia` service binary, which in turn discovers
`sepia-driver-*` binaries itself) and campaigns (`peregrin-driver`
impls + `peregrin-engine`, in-process).

## Cross-repo integration: sepia as a managed subprocess

peregrin does **not** link sepia's session plane as library crates.
Instead the `sepia` binary exposes a service mode — ndjson JSON-RPC
over stdio, the same wire style `sepia-driver-*` binaries already use —
and `peregrin-node` discovers/spawns/respawns it exactly like a driver:

- Symmetric with the existing model: "the session plane is just
  another managed binary." Upgrade sepia without rebuilding peregrin.
- Sepia stays independently deployable and the boundary is
  language-agnostic — third parties get the same interface peregrin
  uses.
- Shared code shrinks to wire types: peregrin takes a **git dep on
  `sepia-proto`** (pure data crate, pinned branch/tag). Everything else
  travels the wire.
- Compat contract = the wire: manifest/capabilities handshake at
  spawn, version negotiation; wire changes are coordinated changes.
- `sepia serve` (or `sepia io`/`--stdio` verb): design reuses the
  `sepia-driver` serve-loop machinery. [OPEN: verb name + handshake]
- Optional dev aid: `.cargo/config.toml.example` showing `[patch]` →
  `../sepia` for local wire iteration.
- crates.io publishing later, when the bupha release model is adopted.

## Migration steps

### Phase 0 — prep

- peregrin: resolve dirty `main` (uncommitted `game/` + `adrs/0007`
  deletions — commit or discard). [needs user decision]
- sepia: confirm `rust-rewrite` clean and pushed.

### Phase 1 — branches

- sepia: `git switch -c v2` from `rust-rewrite`.
- peregrin: `git switch -c v2` from `main`.
- Merge-day target: peregrin `v2 → main`; sepia — decide then whether
  `v2 → master` or `v2` supersedes `rust-rewrite` as the dev line. [OPEN]

### Phase 2 — scaffold

Apply the bupha anatomy to both repos (root files, workspace
Cargo.toml, xtask, docs skeleton, project/, skills, CI, dist config).

### Phase 3 — sepia reshape

1. Rename: `sepia-driver-sdk → sepia-driver`,
   `sepia-driver-host → sepia-host`.
2. Split `sepia-control`: mechanism → new `sepia-session`;
   `merged.rs` + ownership bookkeeping → extracted for peregrin.
3. `git rm` the app crates; slim `sepia-cli` to local verbs.
4. Add `sepia-agents` (agent catalog / config+rules+skills discovery).
5. Prune workspace members + deps (leptos/axum/thirtyfour/keyring
   leave; keep what `sepia-session` needs).
6. Rewrite README (library+CLI story), AGENTS.md, docs skeleton;
   `DEPLOY.md` content migrates into peregrin docs.
7. `cargo xtask check && cargo xtask test` green.

### Phase 4 — peregrin scaffold + import

1. Remove TS tree on `v2` (`package.json`, `bun.lock`, `apps/`,
   `packages/`, `tsconfig.json`, `vite.config.ts`, `.vite-hooks`,
   `.windsurfrules`, `skills-lock.json`; keep `libraries/`, `project/`,
   `adrs/` → fold into `project/adrs/`? [OPEN], `repos/`? [OPEN:
   still needed as vendored Effect reference during the port]).
2. Scaffold Rust workspace per anatomy.
3. Copy app crates from sepia `rust-rewrite` (a worktree of that
   branch), rename crates/idents/env vars, re-point domain deps at
   `sepia-*` crates (git dep + local patch).
4. `peregrin` binary: `serve`/`hub`/`pair`/`service`/`sessions` verbs
   wired to imported crates.

### Phase 5 — platform crates

Skeleton ports first (traits + types + thin impls), then port engine
semantics from the TS code + `project/adrs`. Drivers:
shell/http/agent/human first; webhook/polling/stdin/streaming per
priority. `peregrin-bus` local impl; `peregrin-remote-http` first
transport; p2p impl is its own ADR (iroh vs libp2p — later).

### Phase 6 — glue + verify + publish prep

- `peregrin-node` composes `sepia-session` + `peregrin-engine`;
  `peregrin-hub` merges/routes/serves `peregrin-web`.
- Both repos: `cargo xtask check && cargo xtask test` green, browser
  smoke for the app, e2e node+hub stack.
- Draft + get approval for new GitHub repo descriptions/topics/website
  fields. Proposed:
  - sepia: *"Coding-agent sessions, agnostically — one canonical IR,
    store drivers, and an ACP client for every agent. Rust."*
  - peregrin: *"Event-driven orchestration for agent workflows —
    nodes, campaigns, and remote transport across your machines."*
- Merge `v2` per repo when satisfied.

## Resolved decisions

- Two binaries in peregrin: `peregrin` (cli + `serve` = node) and
  `peregrin-hub` (hub + web UI) — daemon machines skip the web stack.
- peregrin↔sepia: managed subprocess over ndjson JSON-RPC, not crate
  linking (only `sepia-proto` is a cargo dep).
- Tailwind v4 pipeline stays for `peregrin-web` (`bun run css` via
  xtask) — "no TypeScript" means no TS code, the CSS toolchain remains.
- Secrets: sturn/bupha port+adapter pattern (`-secret` port,
  `-keyring`/`-dotenv` impls).
- Driver dir resolution: platform-agnostic (XDG now, win/mac backlog).
- Coverage: 100% goal, llvm-cov + conformance suites gate it.
- Reference material (`repos/`, `rules/`, old docs) may be kept during
  the port and pruned later once unused.
- Docs live on dedicated `docs` branches (worktree at `docs/`), like
  bupha; diataxis skill reinstalled via skills.sh in both repos.

## Open decisions

1. `sepia-session` vs keeping the `sepia-control` name for the
   mechanism crate.
2. `sepia-host` vs `sepia-driver-host` naming.
3. `transfer` (cross-node session copies): `sepia-session` feature or
   own crate.
4. `sepia` service-mode verb name + spawn handshake design.
5. Env prefix: `PEREGRIN_*` vs something shorter.
6. `adrs/` top-level vs `project/adrs/` consolidation in peregrin.
7. sepia merge target on merge day (`master` vs `rust-rewrite` line).
8. PWA/push: carried over as-is (recommended) or re-scoped.
9. Telemetry: `sepia-telemetry` is a library crate — if peregrin only
   talks stdio to sepia it can't import it; copy to
   `peregrin-telemetry` or extract shared?
