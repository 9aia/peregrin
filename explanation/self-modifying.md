# Self-modifying workflows

A node emits an event whose payload _is_ new node specs. The
`runner` driver injects them into the live graph — `ctx.inject` is
the primitive, `driver: runner` the off-the-shelf user of it. This
lets a workflow grow at runtime instead of declaring every step in
advance.

## Why self-modify?

Some workflows cannot know the full graph up front. A planning step
may discover how many tasks are needed. A refinement loop may add
verification nodes. A dispatch step may fan out to a variable
number of workers.

## Basic injection

```yaml
nodes:
  - id: planner
    driver: shell
    on: [start]
    emits: [plan.ready]
    params:
      cmd: "plan.sh > plan.json && cat plan.json"   # produces node specs

  - id: absorb
    driver: runner
    on: [plan.ready]
    emits: [absorb.done]
```

The `runner` driver reads node specs from the trigger's
`payload.nodes` (or `payload.workflow.nodes`) and calls
`ctx.inject`; the bus registers each spec as a node — `Node::from_dict`
is total, so even a partial spec registers — and the new nodes begin
matching events on the next publish. `runner` itself emits
`{injected: N}`.

Specs may arrive as a JSON array, as a JSON *string* on
`payload.nodes`, or as `payload.stdout` containing a bare `[...]` or
`{nodes: [...]}` — so a `driver: shell` planner like the `cat
plan.json` above feeds `runner` directly (`libraries/map/` builds
its fan-out on exactly this).

## Scoping injected nodes (`retire_on`)

Injected nodes accumulate in the graph forever — unless they clean
themselves up:

```yaml
- id: temp-worker
  on: [task.assigned]
  emits: [task.done]
  retire_on: [task.done]
```

`retire_on` is exact-match: once any listed type has been seen, the
node is removed from the live graph entirely (the observer sees
`on_retire`). Unlike `until` — which stops a node from firing but
leaves it registered — `retire_on` deletes it. This prevents
unbounded growth in self-modifying workflows that inject in a loop.

## The trust note

Injection is how an AI generator gets nodes into the graph — and
injected node specs carry their own `capabilities:`. The
dispatch-time capability check still runs on every injected node
(the campaign boundary holds), but inject does **not** narrow an
injected node's caps to the injecting node's. Strip caps off
generated nodes rather than trusting self-attestation — the full
analysis is in [Threat model](threat-model.md) and
[Safety](safety.md).

## See also

- [Loop protection](loop-protection.md) — `max_fires`, `until`,
  `retire_on`
- [Drivers](drivers.md) — `ctx.inject` and the `runner` driver
- [Campaign schema](../reference/campaign-schema.md) — what a node
  spec may carry
