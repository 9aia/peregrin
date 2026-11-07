# Event graph model

## Why events, not DAGs

A DAG (directed acyclic graph) encodes a fixed execution order. You
define edges: A → B → C. The runtime walks the graph topologically.

This works for pipelines but breaks down when:

- **Branching depends on results** — you do not know which path to
  take until a node runs. In a DAG, you would need conditional edges
  or pre/post hooks.
- **The graph changes at runtime** — a node decides it needs 3 more
  tasks. In a DAG, you would need dynamic subgraphs or a meta-layer.
- **Multiple consumers of the same output** — one node result feeds
  3 downstream nodes. In a DAG, that is fan-out, but the DAG does not
  know they are independent.

## The event graph

In an event graph, there are no edges. Nodes subscribe to event
types (`on`) and emit event types (`emits`). The topology emerges
from the contracts:

```
Node A: on=[start], emits=[a.done]
Node B: on=[a.done], emits=[b.done]
Node C: on=[a.done], emits=[c.done]
```

The "edge" A → B is implicit: A emits `a.done`, B subscribes to
`a.done`. The bus matches them at runtime. No one declares the edge.

## What this gives you

**Branching is free.** A driver decides what to emit based on its
result — `params.emit` maps return codes onto different event types.
The graph needs no conditional edges.

**Fan-out is free.** Every node whose `on` matches an event is a
candidate for the same fire round. The bus is the scheduler.

**Fan-in is free.** `mode: all` waits for every `on` pattern to have
a match in the log; `fan_in:` is the count-based join over one
pattern. No join nodes or synchronization primitives — see
[`on:` event patterns](../reference/on-grammar.md).

**Dynamic topology is free.** A driver can emit events that trigger
nodes that did not exist when the run started. `ctx.inject` (and the
`runner` driver built on it) adds nodes to the live graph. See
[Self-modifying workflows](self-modifying.md).

**Wildcards are free.** `on: [shard.*.ready]` catches any
`shard.N.ready` event — map-reduce without fixed width; the planner
decides at runtime how many shards exist. Captures ride into
templates as `${event.match.N}`.

## The core loop

The `Runner` (`peregrin-engine`) is a sequential, queue-driven graph
walker:

```
1. Publish `start` (or `seed` a replay log + `check_ready`)
2. Each loop iteration: drain due poll ticks, drain the ingress
   channel into `pending`
3. Run every `ready` (node, triggers) pair in order; collect each
   node's returned events, `ctx.emit` side events, `ctx.inject` specs
4. Publish pending events → matched nodes become `ready` → loop
5. `workflow.cancel` or a `stop_on` type → `workflow.cancelled`;
   a failure anywhere → `workflow.failed`; queues drain cleanly →
   `workflow.done`
```

v1 interleaved `ready` batches through `Effect.forEach` with
unbounded concurrency, but all cross-node state was only touched at
yield points — the sequential batch walk is semantically equivalent
and deterministic. Timed work doesn't sleep the loop: poll `<id>.tick`
re-fires and driver-deferred emissions (`ctx.schedule_at`, ADR-0018)
sit on a deadline heap that drains at the top of each iteration —
deferred events also hold a finite run open until they land.

The ingress channel is how external producers (webhook listeners,
WAL pollers, the supersede queue's cancel signal) feed a live run
without `&mut` access to the bus — injected events serialize at the
next iteration boundary. See
[Queue & supersede](../reference/queue-supersede.md).

## The primitives

```
Event    — the flat envelope: type, payload, source, id, ts.
           The unit of flow and state (peregrin-core).
Campaign — name, config, capabilities, the node list (peregrin-core).
Node     — subscribes (on) + produces (emits) + runs via a driver.
Driver   — executes the action (shell, agent, git, …) — one unified
           registry, ingress and sinks included.
Bus      — LocalBus matches published events to subscriptions,
           returns (node, triggers) pairs (peregrin-bus).
Runner   — owns the loop, capability checks, retry/circuit, timers,
           ingress (peregrin-engine).
Store    — the event log + run state + DLQ ports (peregrin-store).
Observer — lifecycle hooks (peregrin-bus).
```

**Branching lives in the driver** (it decides what events to emit).
**Topology lives in the graph** (on/emits contracts). Clean
separation: the graph says _when_, the driver says _what happened_.
