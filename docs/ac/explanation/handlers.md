# Handlers

Handlers are Python functions that implement node logic. They are the seam
between the graph (data) and execution (code). Most logic should live in
shell templates — Python handlers exist for cases that need real computation.

## Handler contract

```
handler(triggers: list[Event], ctx: Context, node: Node) -> list[Event]
```

- `triggers` — the events that caused this node to fire. For `mode: any`,
  it is a list with one event (the matching one). For `mode: all`, it is a
  list with one event per `on` pattern.
- `ctx` — the context object (config, secrets, log, emit, inject).
- `node` — the node definition (id, with_, etc.).

Returns a list of events to emit. Can be empty (emit nothing).

## Why use Python handlers?

Python is the right choice when a node needs to:

- Branch in ways that cannot be expressed with `emits` exit-code mapping.
- Call an external API and transform the response.
- Plan dynamic work and inject new nodes.
- Maintain state across the event log.

## When not to use them

For simple glue — shelling out, fetching data, sending notifications — the
`shell` or `http` drivers are usually enough. Prefer the simplest driver
that can do the job.

## See also

- [Handler context](../reference/handler-context.md) — the `ctx` object
- [Handler registration](../reference/handler-registration.md) — by name or module path
- [Custom drivers and handlers](../how-to/custom-drivers.md) — recipes and examples
