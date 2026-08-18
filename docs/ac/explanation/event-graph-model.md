# Event graph model

## Why events, not DAGs

A DAG (directed acyclic graph) encodes a fixed execution order. You define
edges: A → B → C. The runtime walks the graph topologically.

This works for pipelines but breaks down when:

- **Branching depends on results** — you do not know which path to take until
  a node runs. In a DAG, you would need conditional edges or pre/post hooks.
- **The graph changes at runtime** — a node decides it needs 3 more tasks.
  In a DAG, you would need dynamic subgraphs or a meta-layer.
- **Multiple consumers of the same output** — one node result feeds 3
  downstream nodes. In a DAG, that is fan-out, but the DAG does not know
  they are independent.

## The event graph

In an event graph, there are no edges. Nodes subscribe to event types
(`on`) and emit event types (`emits`). The topology emerges from the
contracts:

```
Node A: on=[start], emits=[a.done]
Node B: on=[a.done], emits=[b.done]
Node C: on=[a.done], emits=[c.done]
```

The "edge" A → B is implicit: A emits `a.done`, B subscribes to `a.done`.
The bus matches them at runtime. No one declares the edge.

## What this gives you

**Branching is free.** A node handler decides what to emit based on its
result. The graph does not need conditional edges — the handler just emits
different event types.

**Fan-out is free.** Multiple nodes subscribing to the same event run in
parallel automatically. The bus is the scheduler.

**Fan-in is free.** `mode: all` on a node with multiple `on` patterns
waits for all of them. No join nodes or synchronization primitives.

**Dynamic topology is free.** A handler can emit events that trigger nodes
that did not exist when the workflow started. The `runner` driver injects
new nodes at runtime. The graph grows organically.

**Wildcards are free.** `on: [shard.*.ready]` catches any `shard.N.ready`
event. Map-reduce without fixed width — the planner decides at runtime
how many shards to create.

## The core loop

```
1. Emit `start` event
2. Bus finds nodes whose `on` matches → returns (node, triggers) pairs
3. Runner runs all ready nodes concurrently (asyncio.gather)
4. Nodes emit new events
5. Publish emitted events → back to step 2
6. No more ready nodes → emit `workflow.done`
```

The event log (append-only) IS the workflow state. Every event is recorded.
The log is replayable, inspectable, and persistent.

## The primitives

```
Event   — typed message with a payload. The unit of flow and state.
Node    — subscribes (on) + produces (emits) + runs a handler via a driver.
Driver  — executes the handler (shell, python, runner).
Bus     — matches emitted events to subscriptions (wildcards), returns ready nodes.
Runner  — owns the loop, runs ready nodes concurrently, publishes results.
Store   — persists the event log (adapter port).
Observer— receives lifecycle hooks (adapter port).
```

**Branching lives in the handler** (it decides what events to emit).
**Topology lives in the graph** (on/emits contracts). Clean separation:
the graph says _when_, the handler says _what happened_.
