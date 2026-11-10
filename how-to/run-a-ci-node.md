# Run a CI runner node

GitHub Actions runners (or any ephemeral host) can join the entity
plane as first-class nodes: boot `peregrin serve --ephemeral`, let the
hub schedule work onto it, and on shutdown the node flushes snapshots
of everything still live so nothing dies with the runner.

## Boot

```bash
peregrin serve --ephemeral
# or equivalently:
PEREGRIN_EPHEMERAL=1 peregrin serve
# the standalone daemon reads the env var too:
PEREGRIN_EPHEMERAL=1 peregrin-node
```

Minimum env for a runner:

```bash
export PEREGRIN_HOST=0.0.0.0          # reachable from the hub
export PEREGRIN_PORT=18787
export PEREGRIN_TOKEN=<shared-secret>  # REQUIRED off-loopback
export PEREGRIN_EPHEMERAL=1
export PEREGRIN_HOME=$RUNNER_TEMP/peregrin   # per-job home
```

## Auth — no pairing needed

Node→hub trust is **configuration, not pairing**. The hub's operator
lists the runner in `PEREGRIN_NODES` like any other node — for
`peregrin-hub` that is the only variable it reads (`HYPHA_NODES` is the
standalone `hypha-hub` binary's contract):

```bash
# on the hub
PEREGRIN_NODES="durable=http://127.0.0.1:18787@tok1;ci=$RUNNER_URL@tok2"
```

The runner itself needs only `PEREGRIN_TOKEN` — the bearer clients
(hub, CLI) present. `peregrin pair` is a *client↔node* web-UI flow and
plays no part in a node joining a hub; ephemeral mode does not weaken
or bypass any auth — a non-loopback bind without `PEREGRIN_TOKEN`
still refuses to start.

## What survives shutdown

On SIGINT **or SIGTERM** (what orchestrators actually send) the node:

1. stops accepting connections (graceful drain),
2. exports a snapshot of every entity that is `live` or `locked` on
   every registered kind plane (`sepia-session`, `campaign`, …),
3. closes the session plane as usual.

Snapshots land in two places, both best-effort:

- **Local**: `$PEREGRIN_SNAPSHOT_DIR/<kind>/<id>.snapshot.json` (default
  `$PEREGRIN_HOME/snapshots`) plus an `index.json` manifest. On CI,
  point `PEREGRIN_HOME` at a workspace volume or upload the snapshots
  dir as a job artifact.
- **Handoff**: set `PEREGRIN_SNAPSHOT_TARGET` to a surviving peer node
  (or the hub) and each snapshot is POSTed to
  `POST /api/entities/snapshot` during shutdown, so the entity resumes
  where it can keep running:

  ```bash
  export PEREGRIN_SNAPSHOT_TARGET=http://durable-node:18787
  export PEREGRIN_SNAPSHOT_TOKEN=<that node's bearer>   # optional;
                                                      # falls back to PEREGRIN_TOKEN
  # when the target is a HUB, pick the import node explicitly:
  export PEREGRIN_SNAPSHOT_NODE=durable
  ```

  The hub stores no snapshots itself — its import route forwards to a
  registered node, which is why the target can be "a node or a hub".

Entity ops queued hub-side in the durable outbox already survive a
node dying — the flush covers entity *state* (session IR, campaign
run checkpoints), which only the node itself can export.

## Locks

No explicit lock cleanup is needed: the hub's lock book is in-memory
and arbitrates stale holds (`hypha-control`, ~30s stale threshold) —
a dead runner's holds preempt automatically, and the session plane's
own lock TTLs/`close_all` already run on every shutdown.

## What ephemeral does not change

Auth, the `/api/*` surface, node identity persistence
(`PEREGRIN_NODE`/`PEREGRIN_HOME` behave normally), and non-ephemeral
shutdown behavior are all identical — the flag only adds the
SIGTERM-aware shutdown and the snapshot flush. `peregrin serve`
without the flag is byte-for-byte the old behavior.
