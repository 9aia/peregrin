# Loop protection

Event graphs can loop — a node emits an event that eventually
re-triggers itself. That is a *feature* (retries, refinement,
poll-until-condition) and a *risk* (an unbounded ping-pong burns a
machine). The bounds, from specific to general:

## 1. Per-node: `max_fires` + `until` + `retire_on`

**`max_fires`** — bounds how many times a node can fire. Default 1
(fire once); `max_fires: unlimited` opts out. This is the primary
mechanism for intentional loops: a fix node with `max_fires: 3`
fires at most 3 times, then stops — regardless of what events are
still flowing. (`0`, negative, and non-integer values fall back to
the default — the v1 quirk the port keeps; see
[Campaign schema](../reference/campaign-schema.md).)

When a node hits its cap and another matching event arrives, the
bus emits `node.exhausted` — at most once per node per run — so a
workflow can *react* to exhaustion instead of silently quiescing to
an ambiguous `workflow.done`. A node stopped by `until` does not
emit it: that is the intended stop, not exhaustion.

```yaml
- id: fix
  driver: agent
  on: [review.rejected]
  emits: [fix.done, fix.failed]
  max_fires: 3

- id: fix-exhausted
  driver: shell
  on: [node.exhausted]
  emits: [workflow.aborted]
  params:
    cmd: |
      echo "FIX EXHAUSTED — node '${event.payload.node}' hit max_fires=${event.payload.maxFires}"
```

**`until`** — declarative termination. Stops a node from firing
once any listed event type exists in the log. "Keep going until
done." Exact-match, not glob.

```yaml
max_fires: unlimited
until: [op.success]
```

**`retire_on`** — removes the node from the live graph entirely.
Unlike `until` (the node stays in the graph but stops firing),
`retire_on` deletes it — for scoping injected nodes that would
otherwise accumulate forever.

**`stop_on`** — the campaign-level counterpart: an exact-match type
list that ends the whole run as `workflow.cancelled`.

## 2. Cycle detection — not ported

v1 watched the event log and aborted a run whose last N events
repeated K times in a row (`workflow.cycle_detected`). The event
type and the observer `cycle` flag exist in the port, but the
detector itself does not — nothing in `peregrin-engine` emits
`workflow.cycle_detected` today.

> Until it lands, the guards are explicit: keep `max_fires` bounded
> on every loop-creating node (default 1 already is), use `until`
> for poll-until patterns, and treat an accidental cycle as a
> `max_fires`-exhaustion, not a hang.

## 3. Global bounds — what exists today

v1's global `max_events`/`--timeout`/`--max-concurrency` run caps
are likewise unported (`RunOptions` carries only `dry_run`,
`secrets`, `replay_events`). The bounds that do exist:

- **Driver timeouts** — `shell` defaults to 30 s capped at 600;
  `agent` 600 s (0 = none); `http` 300 s; `human` 300 s (0 = wait
  forever); `github-status` 30 s capped at 300.
- **Retry + circuit breaker** — `retry.max_attempts` with
  `fixed`/`linear`/`exponential` backoff; `circuit_breaker` opens
  after `threshold` consecutive failures for `reset_after` seconds.
  See [Resilience](resilience.md).
- **Supersede + cancel** — under `cancel_running`, a newer trigger
  for the same supersede key pushes a `workflow.cancel` into the
  active run's ingress channel; the run honours it at the next
  iteration boundary. See
  [Queue & supersede](../reference/queue-supersede.md).
- **`ctx.request_shutdown()`** — a driver can cancel the run after
  the current batch.

## Intentional vs accidental

| Type | How to express | Protection |
| --- | --- | --- |
| Retry N times | `max_fires: N` + `on: [node.exhausted]` | `max_fires` |
| Poll until condition | `poll:` + `until: [condition.event]` | `until` |
| Refinement loop | `max_fires: N, until: [good.enough]` | both |
| Clean up injected node | `retire_on: [done.event]` | `retire_on` |
| End the run on a verdict | `stop_on: [verdict.failed]` | `stop_on` |
| Accidental cycle | (none — you forgot a bound) | `max_fires` default; cycle detection *pending* |
| Stuck handler | (driver-level) | `params.timeout` caps |

The principle: **express what you want explicitly; the runner
catches what you did not** — and where the catchers aren't ported
yet, explicit bounds carry the whole weight.
