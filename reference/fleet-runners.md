# Fleet runners (`runs_on`)

Peregrin nodes can advertise **runner labels** and campaigns can
declare which labels they require. Scheduling rides Hypha's entity
plane — campaign entities, `campaign.start` ops, ownership and
locks — never a separate lease protocol.

## Node labels

`PEREGRIN_RUNNER_LABELS` is a comma-separated list appended to the
node's `/api/node` `capabilities` (alongside `ephemeral`, `sessions`,
…):

```bash
PEREGRIN_RUNNER_LABELS=runner:linux,gpu peregrin serve --ephemeral
PEREGRIN_RUNNER_LABELS=runner:linux,gpu peregrin campaign serve ci-gate.yaml
```

The hub's sync projection persists the advertised capability list,
so fleet members are discoverable via node status.

## Campaign `runs_on`

```yaml
name: gpu-train
runs_on: [runner:linux, gpu]
nodes: [...]
```

Every `runs_on` entry must appear in the node's advertised
capabilities (AND semantics; absent `runs_on` runs anywhere). The
gate is enforced in two places:

- **`campaign.start` entity ops** — the campaign entity plane on the
  owning node rejects a start whose `runs_on` the node doesn't
  satisfy (`409`), so a misrouted op fails loudly instead of running
  on the wrong hardware. Combined with Hypha ownership + locks, only
  the entity's owner can launch it — two nodes can't claim the same
  campaign entity.
- **`campaign serve` hosting** — campaigns whose `runs_on` isn't
  satisfied are skipped at startup (warned); if none match, serve
  exits with an error rather than hosting half a fleet.

## Job isolation

`driver: shell` params accept:

- `isolate: true` — run the command inside
  `systemd-run --user --scope --collect`.
- `memory_max: "512M"` — adds `-p MemoryMax=…` and implies `isolate`.

Both require `systemd-run` on `PATH` and a working `systemd --user`
manager; without them the node fails with `node.failed` — isolation
is never silently dropped. `bupha`-imported jobs carry `isolate` /
`memory_max` through to the inner shell driver automatically.

Deferred: cgroup v2 direct manipulation, container/VM isolation,
per-job CPU/IO limits beyond `MemoryMax`.

## Ephemeral-node recovery

An ephemeral runner (`--ephemeral` / `PEREGRIN_EPHEMERAL`) that dies
mid-run is reaped by the hub after `PEREGRIN_EPHEMERAL_DEAD_AFTER`:
its projected entities drop and outbox writes queued for it
dead-letter. Campaign entities hosted by `campaign serve` keep their
durable SQLite queue — on restart, interrupted runs requeue at the
front per the queue's at-least-once semantics, and `supersede` keys
still dedupe retriggered work.
