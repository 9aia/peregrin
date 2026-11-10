# Run a CI runner fleet

> **Deprecated (GitHub Actions removal):** the workflows this page
> references (`peregrin-ci.yml`, `ephemeral-fleet.yml`,
> `install-peregrin`) were deleted — the org no longer uses GitHub
> Actions. The *concept* below (ephemeral `peregrin serve --ephemeral`
> nodes joining a long-lived hub) still applies to any ephemeral host
> (bare runners, cloud VMs, containers); only the GHA-specific pieces
> are gone. See [run-a-ci-node](./run-a-ci-node.md) for the node side.

A *fleet* is N ephemeral peregrin nodes booted on CI runners, joined
to a hub that outlives them. GitHub Actions ships the pieces:

- `.github/workflows/peregrin-ci.yml` — the **reusable workflow** any
  repo can call for a single CI node or a local campaign run.
- `.github/workflows/ephemeral-fleet.yml` — a `workflow_dispatch`
  **example** that boots a matrix of runners as nodes.
- `.github/actions/install-peregrin` — the shared install step
  (release binary when published, `cargo install --git` until then).

## Topology — read this first

```
        hub (your machine / server / long-lived runner)
         │  PEREGRIN_NODES="ci-1=http://…:18787@tok;ci-2=…"
         │  ▲ hub→node ops only work if the hub can REACH the node
         │
   ┌─────┴──────────────────────────┐
   │  runner ci-1   runner ci-2 …   │   GitHub-hosted: NO inbound
   │  peregrin serve --ephemeral    │   connections — hub cannot dial
   └─────┬──────────────────────────┘   them; useful directions are
         │ outbound: snapshots → hub,  │   OUTBOUND only:
         │ `peregrin` CLI → hub/node   │
```

- **Hub stays home.** It owns the entity plane view, the lock book,
  and the outbox. Ephemeral runners join it as ordinary nodes —
  auth is config (`PEREGRIN_NODES=id=url[@token];…` on the hub,
  `PEREGRIN_TOKEN` on each node), never pairing.
- **Runners are ephemeral nodes.** On SIGTERM each flushes snapshots
  of `live`/`locked` entities — locally under
  `PEREGRIN_SNAPSHOT_DIR` (uploaded as a job artifact) and/or handed
  off to `PEREGRIN_SNAPSHOT_TARGET` so the entity resumes where it
  can keep running.
- **Ops route via ownership.** The hub arbitrates locks
  (`hypha-control`, ~30s stale threshold) and forwards ops to the
  node that owns an entity — *if it can reach it*. On GitHub-hosted
  runners that means outbound-only: snapshots and client verbs. For
  real hub→runner op routing use **self-hosted runners on a
  tailnet/LAN** the hub can dial.

## Single job — the reusable workflow

Any repo calls it; the workflow checks out *your* repo, installs
peregrin, and runs one of two modes:

```yaml
jobs:
  campaign:
    uses: 9aia/peregrin/.github/workflows/peregrin-ci.yml@main
    with:
      mode: campaign-run
      campaign: path/to/campaign.yaml
      campaign-args: --dry-run

  node:
    uses: 9aia/peregrin/.github/workflows/peregrin-ci.yml@main
    with:
      mode: ephemeral-node
      command: peregrin sessions list   # any shell; node env is set
      node-id: ci-worker                # stable id for PEREGRIN_NODES
      snapshot-target: http://hub.tailnet:13000
      snapshot-node: home-node          # hub-side import target
    secrets:
      PEREGRIN_TOKEN: ${{ secrets.PEREGRIN_TOKEN }}
      PEREGRIN_SNAPSHOT_TOKEN: ${{ secrets.PEREGRIN_SNAPSHOT_TOKEN }}
```

`ephemeral-node` boots `peregrin serve --ephemeral` on
`127.0.0.1:$port`, waits for `/api/health`, runs `command` (with
`PEREGRIN_NODE_URL`/`PEREGRIN_TOKEN` in env), then SIGTERMs — the
flush runs even when `command` fails (`if: always()`). Snapshots are
uploaded as a `peregrin-snapshots-*` artifact.

Secrets are never echoed: `PEREGRIN_TOKEN`,
`PEREGRIN_SNAPSHOT_TOKEN`, `HYPHA_OTEL_ENDPOINT`,
`HYPHA_OTEL_SERVICE_NAME`. See the header comment in
`peregrin-ci.yml` for the full input list.

## The fleet example

`.github/workflows/ephemeral-fleet.yml` is `workflow_dispatch`-only —
dispatch it from the Actions tab:

| input | meaning |
| --- | --- |
| `nodes` | JSON array of node ids — `["ci-1","ci-2"]`. Each becomes the seeded `node.json` id the hub's `PEREGRIN_NODES` names. |
| `hub-url` | Snapshot handoff target **and** the URL `peregrin` CLI verbs use. |
| `snapshot-node` | Which registered node the hub imports snapshots onto (required for hub handoff — the hub stores no snapshots itself). |
| `hold-seconds` | How long nodes stay up. |
| `peregrin-ref` | Git ref to install (branch/tag/SHA). |

Repo secrets: `PEREGRIN_TOKEN` (one shared bearer — every node accepts
it and the hub presents it), `PEREGRIN_SNAPSHOT_TOKEN` (what the
handoff target expects; falls back to `PEREGRIN_TOKEN`), optional
`PEREGRIN_HUB_TOKEN`. Optional var `PEREGRIN_FLEET_HOST` overrides the
`0.0.0.0` bind.

Each matrix leg seeds `$PEREGRIN_HOME/node.json` with its matrix id
**before first boot**, so the hub can list `ci-1=http://…@tok` in
`PEREGRIN_NODES` instead of chasing auto-minted `node_<hex>` ids.

## Installing peregrin on the runner

Until published releases exist (tracked as T-003), the install step
runs:

```bash
git config --global url."https://github.com/".insteadOf "ssh://git@github.com/"
cargo install --git https://github.com/9aia/peregrin.git --locked --bin peregrin --branch <ref>
```

Two things to know:

- **The ssh→https rewrite is required.** peregrin's manifest pins
  `sepia`/`tamias`/`hypha` crates as `ssh://git@github.com/…` deps;
  runners have no SSH keys. The repos are public, so https works.
- **It compiles the dependency graph** — expect several minutes on a
  cold cache (`Swatinem/rust-cache` helps across runs). Once T-003
  ships release tarballs, the same step downloads
  `peregrin-x86_64-unknown-linux-gnu.tar.gz` instead — no workflow
  change needed.

## Session plane honesty

`PEREGRIN_SESSION_PLANE=local` (the default in both workflows) links
`sepia-session` **in-process** — the node boots with only the
`peregrin` binary, no `sepia` install. Campaign and substrate work
(`kind="campaign"`, entities, ops, snapshots) needs nothing more.
Real agent sessions still need the sepia **drivers**
(`sepia-driver-*` on `PATH`) and, if you unset the plane override,
the `sepia` binary itself (`PEREGRIN_SEPIA` or PATH).

## See also

- [Run a CI runner node](run-a-ci-node.md) — the single-node
  mechanics (env vars, snapshot destinations, locks).
- [Run a hub](run-a-hub.md) — `PEREGRIN_NODES` config on the hub side.
