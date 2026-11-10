# Run a CI runner fleet

A *fleet* is N ephemeral peregrin nodes booted on throwaway hosts
(CI runners, cloud VMs, containers), joined to a hub that outlives
them. The org no longer uses GitHub Actions — the reusable-workflow
examples this page once carried are gone — but the topology is
host-agnostic: any orchestrator that can start a process and set env
vars can boot a node.

## Topology — read this first

```
        hub (your machine / server / long-lived host)
         │  PEREGRIN_NODES="ci-1=http://…:18787@tok;ci-2=…"
         │  ▲ hub→node ops only work if the hub can REACH the node
         │
   ┌─────┴──────────────────────────┐
   │  runner ci-1   runner ci-2 …   │   hosted CI: NO inbound
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
  `PEREGRIN_SNAPSHOT_DIR` (persist it as a job artifact or on a
  volume) and/or handed off to `PEREGRIN_SNAPSHOT_TARGET` so the
  entity resumes where it can keep running.
- **Ops route via ownership.** The hub arbitrates locks
  (`hypha-control`, ~30s stale threshold) and forwards ops to the
  node that owns an entity — *if it can reach it*. On hosted CI
  runners that means outbound-only: snapshots and client verbs. For
  real hub→runner op routing use self-hosted runners on a
  tailnet/LAN the hub can dial, or have the runner dial out.

## Booting a node

The per-node mechanics — env vars, snapshot destinations, lock
behavior — are in [Run a CI runner node](run-a-ci-node.md). The
short version for any orchestrator:

```bash
export PEREGRIN_HOST=0.0.0.0
export PEREGRIN_PORT=18787
export PEREGRIN_TOKEN=<shared-secret>     # REQUIRED off-loopback
export PEREGRIN_EPHEMERAL=1
export PEREGRIN_HOME=$RUNNER_TEMP/peregrin
# optional handoff:
export PEREGRIN_SNAPSHOT_TARGET=http://hub:18787
export PEREGRIN_SNAPSHOT_NODE=home-node    # when the target is a hub
peregrin serve --ephemeral
```

Seed `$PEREGRIN_HOME/node.json` with a stable id **before first
boot** so the hub's `PEREGRIN_NODES` can name it (`ci-1=…`) instead
of chasing auto-minted `node_<hex>` ids.

## Installing peregrin on the runner

Until published releases exist, install from source:

```bash
git config --global url."https://github.com/".insteadOf "ssh://git@github.com/"
cargo install --git https://github.com/9aia/peregrin.git --locked --bin peregrin --branch <ref>
```

Two things to know:

- **The ssh→https rewrite is required.** peregrin's manifest pins
  `sepia`/`tamias`/`hypha` crates as `ssh://git@github.com/…` deps;
  runners have no SSH keys. The repos are public, so https works.
- **It compiles the dependency graph** — expect several minutes on a
  cold cache. Once release tarballs ship, download
  `peregrin-x86_64-unknown-linux-gnu.tar.gz` instead.

## Session plane honesty

`PEREGRIN_SESSION_PLANE=local` (or `inprocess`) links `sepia-session`
**in-process** — the node boots with only the `peregrin` binary, no
`sepia` install. Campaign and substrate work (`kind="campaign"`,
entities, ops, snapshots) needs nothing more. Real agent sessions
still need the sepia **drivers** (`sepia-driver-*` on `PATH`) and,
if you unset the plane override, the `sepia` binary itself
(`PEREGRIN_SEPIA` or PATH).

## Fleet labels and dispatch

Give runners capabilities the hub can schedule on: set
`PEREGRIN_RUNNER_LABELS=ci,linux` (comma-separated) on each node —
they surface in `/api/node` `capabilities`. Campaigns declare
`runs_on: [label, …]` (AND match) and the hub's fleet dispatcher
routes `campaign.start` ops to eligible nodes, preferring durable
over ephemeral. See [Fleet runners](../reference/fleet-runners.md).

## See also

- [Run a CI runner node](run-a-ci-node.md) — the single-node
  mechanics (env vars, snapshot destinations, locks).
- [Run a hub](run-a-hub.md) — `PEREGRIN_NODES` config on the hub side.
