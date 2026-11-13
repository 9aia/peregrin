# Events

Events are the unit of flow and state. Every event is a flat
envelope — `type`, `payload`, `source`, `id`, `ts` — and the
append-only log the bus keeps IS the workflow state. The full
contract, including the engine-synthetic table (`start`,
`workflow.done`, `node.failed`, `capability.violation`, …), is
specified in [Event envelope](../reference/event-envelope.md);
this page is about the *why*.

## Events are the state

Because the log is append-only and every action is recorded, the
entire run state can be reconstructed by replaying it. That is what
makes the run inspectable (`peregrin campaign run` prints the log,
`--history` persists it), resumable (`--replay` seeds a run from a
prior log and `check_ready` picks up subscriptions that were already
satisfied), and debuggable (`peregrin runs`/`explain`/`log` read the
durable log back).

Two consequences worth internalizing:

- **Run-scoped data rides in `payload`.** The envelope is closed —
  five fields, no more. Repo/sha/ref metadata, state keys, fan-in
  collections: all payload. There is no hidden channel.
- **Nothing secret, ever.** The log is the shareable artifact;
  secrets reach nodes through the run's secrets record and the
  `capabilities.secrets` gate, never through an event.

## Custom events

Custom events are just strings. No registration is required.

### Naming convention

Dot-separated, hierarchical: `pr.approved`, `deploy.success`,
`shard.3.done`.

This is what makes wildcards work: `on: [pr.*]` catches
`pr.approved`, `pr.rejected`, anything with one trailing segment;
`on: [pr.**]` catches the whole `pr.` subtree. Captured segments are
addressable in templates as `${event.match.N}` — which is how
conventions like `push.gated.<repo>.<clone_url>.<sha>` put payload-shaped
data *into* the type itself (the typed-routing idiom (ci-ingress/docs-deploy use it)). See
[`on:` event patterns](../reference/on-grammar.md).

### Emitting

A node's `emits:` list declares its output types; under `strict:
true` every event a node produces must literally appear there.
Where the events actually come from — the returned `Vec<Event>`,
`params.emit` rc-mapping, `ctx.emit`, `forward:` re-emission, and
the ingress channel — is specified in
[Event envelope § Emission contract](../reference/event-envelope.md).

```yaml
- id: review
  driver: shell
  on: [pr.opened]
  emits: [pr.approved, pr.changes_requested]
  params:
    cmd: "review.sh '${event.payload.url}'"
    emit:
      "0": pr.approved
      default: pr.changes_requested
```

## The engine's own events

Engine-synthetic events (`source: "__runner__"`, or the affected
node id for `node.*` types) are exempt from payload-schema
validation and subscribable like any other type — a node can react
to `node.failed`, `node.exhausted`, `circuit.open`, or
`workflow.cancelled` with the same `on:` machinery as domain events.
Failure handling is not a side channel; it is more graph.

## See also

- [Event envelope](../reference/event-envelope.md) — fields,
  synthetic types, emission contract
- [`on:` event patterns](../reference/on-grammar.md) — matching,
  captures, `mode`/`fan_in`
- [Event graph model](event-graph-model.md) — why events are the
  topology
