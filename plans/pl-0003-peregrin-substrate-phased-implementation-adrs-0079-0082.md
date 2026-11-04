+++
id = "PL-003"
title = "peregrin substrate — phased implementation (ADRs 0079-0082)"
status = "open"
created = "2026-11-04"
priority = "high"
+++

# peregrin substrate — phased implementation (ADRs 0079-0082)

Status: **proposed** · Created: 2026-11-04

## Context

Implements the approved ADR set: ADR-0079 (install/remove +
store/cache + verb surface), ADR-0080 (substrate, sink plane,
service scoping, agent surface), ADR-0081 (bupha→peregrin migration
inventory + locked review decisions), ADR-0082 (seven v1
architecture inheritances). Goal: peregrin as the general
orchestration/execution substrate; CI becomes a configuration
artifact (`ci-gate` bundle); bupha freezes as the minimal-box
fallback.

**Standing gates for every phase:** `cargo xtask check` +
`cargo xtask test` green; 100% line coverage on all *new* code
(workspace gate is `--fail-under-lines 100` on touched crates);
`kuda sync` rides code diffs; docs land on the `docs` branch in
diataxis placement; myco statuses honest (task per phase, handoff
on exit); per-phase self-audit (step F below).

## Phases

### P0 — Contracts + v1 spec audit (foundation, no behavior)

- Draft the **event envelope schema**: source, repo, ref, sha,
  clone_url, actor, secret *capability* names, payload. The
  contract every driver/campaign speaks — the last open item from
  ADR-0081.
- Spec `on:` grammar: glob + wildcards (`shard.*.ready`) +
  `mode: all` fan-in (ADR-0082 #1).
- **v1 audit** — diff v1 semantics (`d6bf6356~1:docs/ac/
  explanation/*.md`) against current `peregrin-engine`: which of
  the 7 inheritances already exist in v2, which need building.
  Output: inventory appended to ADR-0082.
- JSON Schema convention for bundle params (field.yml, ADR-0032).
- Exit: envelope + grammar committed as `reference/` docs + an
  ADR-0082 amendment listing exists/build per item.

### P1 — Engine mechanics (queue + graph semantics)

- **Supersede key** in the queue policy — per-campaign
  configurable (key expr, mode: latest-wins/fifo, cancel_running).
- Explicit **worktree lifecycle** helper: created → kept-for-debug
  → destroyed (librarizable — usable outside runs).
- Event-graph gaps from P0 audit: `mode: all` fan-in, wildcard
  subscriptions, `runner` self-modification, 3-layer loop
  protection (`max_fires`/`until`/`retire_on`) — build what's
  missing.
- Exit: unit + e2e coverage; `reference/` docs for queue policy,
  worktree lifecycle, loop protection.

### P2 — Drivers (unified registry, ADR-0082 #4)

- `driver-git` — checkout sha → detached worktree → invoke inner
  driver; uses P1 lifecycle helper.
- Ingress drivers: `driver-github-webhook` (HMAC listener,
  per-repo secrets from node config), `driver-sturn` (WAL ref
  poller — `sturn snap` → run).
- cgroup isolation as a driver option (`isolate: scope`,
  `memory_max`) — the OOM-containment story.
- Secret binding per-impl in node config; campaigns declare
  capabilities (ADR-0081 locked decision).
- Exit: each impl's tests incl. HMAC verification cases + a fake
  WAL poller; `reference/drivers/*` docs.

### P3 — Sinks (egress drivers)

- `sink-github-status` — commit statuses, `status_repo` redirect,
  deterministic `ci/<campaign>` contexts.
- `sink-events`/`sink-sturn` — verdicts back onto the event/WAL
  plane (enables review verdicts, notifications).
- Exit: fake-API e2e tests; `reference/sinks/*` docs.

### P4 — Distribution (ADR-0079)

- `source` port family: `source-fs`, `source-git`
  (`git:<url>[//<subdir>][#<ref>]` grammar — any clone URL,
  `//` split, `#` pin), `source-http` (tarball).
- Store `~/.local/share/peregrin/store/` + content-addressed
  fetch cache `~/.cache/peregrin/fetch/`; resolved-sha recording.
- Verbs: install/remove/list/info/update/pin/verify/cache/
  bundle-pack. **Deep-merge override composition** (ADR-0082 #3):
  `include:` + overlay semantics for bundles.
- Exit: fake-remote e2e installs (git daemon or file:// remote),
  digest-verify tests; `how-to/install-a-bundle` + `reference/`
  grammar docs.

### P5 — `ci-gate` bundle + importer

- `libraries/ci-gate` bundle (manifest + campaigns + params
  schema): on github push → driver-git → inner run →
  sink-github-status.
- `libraries/release` bundle (the bupha release.sh equivalent).
- `bupha.toml`/`bupha.yaml` → campaign-param importer: `repos:` →
  trigger entries; `jobs:` → `on:` filters + driver/sink params;
  must round-trip the live ~70-repo config byte-identical in
  meaning (test fixture = real file).
- Resilience fragments as libs: `retry`, `circuit-breaker`,
  `dead-letter` (ADR-0082 #6) — subscriber-pattern YAML +
  examples + golden tests.
- Exit: importer e2e on a copy of the live config; bundle param
  validation rejects typos (JSON Schema).

### P6 — Ops verbs + observability

- `peregrin create --template <bundle>` (unit + config), `deploy
  webhook`, `destroy webhook|service|worktrees`.
- `runs`/`explain`/`log`/`doctor` equivalents over peregrin-store;
  events feed surfaced.
- Exit: systemd unit integration test (fake or scope), doctor
  parity checks.

### P7 — Agent surface

- `.agents/skills/` (verbs + house rules) + `peregrin-mcp` server
  (sturn-mcp model): install/create/run/runs/explain exposed as
  tools.
- Exit: MCP e2e; skills linted.

### P8 — Shadow → cutover (the actual migration)

- Run peregrin gate beside bupha watcher: same pushes, statuses
  post as `ci/<campaign>` alongside `ci/bupha`.
- Parity window: N real pushes, verdict-diff report (script
  comparing status payloads).
- Cutover: repoint hooks, retire `bupha-watch` unit, freeze bupha
  (release.sh + importer stay), archive note in bupha docs.
- Exit: zero divergence over the window; rollback = hooks back +
  unit re-enable (documented one-command path).

### P9 — Docs sweep + v1 port

- Port `docs/ac/explanation/` (v1) → v2 diataxis explanation
  section; threat-model + safety docs carried over and updated
  for the new plane names.
- Getting-started tutorial for `install`/`create --template`
  (the ci-gate first-run flow).
- Exit: docs deploy job green on the docs push.

## Phase-constant audit step (F)

Every phase closes with: coverage gate verified on touched crates;
kuda sync; myco status update + handoff; self-review of the diff
for convention drift (ports/adapters, deny_unknown_fields,
secret-name-only configs); docs updated in the same phase.

## Risks / watch-items

- **Blast radius** — engine bugs now gate CI; P8's shadow window
  is the control. Rollback stays one command.
- **Envelope churn** — P0 is cheap; if it slips, every later
  phase re-touches consumers. Don't start P2 without it.
- **`9aia/bupha:main` release failures** (303s, pre-existing —
  likely T-044 crates.io token) will mask the shadow window's
  signal on that repo until fixed.
- bupha-ssh drop-orphan test leaks — pre-existing flakes, keep
  out of coverage accounting.

## Module map (vs the 35-crate workspace)

Existing, reused as-is: `peregrin-{core,engine,node,control,
store,bus,sync,outbox,push,session,auth,remote,remote-http,
secret,keyring,dotenv,service,systemd,config,config-toml,
loader,loader-yaml,template,http,hub,web,web-core,telemetry,
driver,driver-{shell,http,agent,human},testkit,cli}`.

New crates by phase:

- P1: engine work only — queue-supersede + graph gaps live in
  `peregrin-engine`; worktree helper in a lib spot (`peregrin-git`
  or inside driver-git — P0 decides).
- P2: `peregrin-driver-git`, `peregrin-driver-github-webhook`
  (ingress — HTTP listener + HMAC), `peregrin-driver-sturn`
  (WAL poller). Isolation option lands on the driver port.
- P3: `peregrin-sink` port + `peregrin-sink-{github-status,sturn}` —
  or unified-driver egress shape if P0 audit prefers one registry
  crate (ADR-0082 #4 says one *registry*, not necessarily one port).
- P4: `peregrin-source` port + `peregrin-source-{fs,git,http}`;
  `peregrin-bundle` (manifest, store layout, deep-merge compose).
- P4 side-car: `peregrin-config-yaml` — node config gets the same
  yaml-first/toml-permanent treatment bupha landed
  (`triggers:`/`campaigns:` keys live here).
- P6: ops verbs land in `peregrin-cli` (+`peregrin-service`/
  `systemd` reuse for deploy/destroy).
- P7: `peregrin-mcp` + `.agents/skills/`.

Open for P0: whether `peregrin-telemetry` already *is* the
observer port (ADR-0082 #7) or the port trait needs extracting
from it.
