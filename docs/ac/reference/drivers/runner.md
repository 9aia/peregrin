# runner driver

Injects new nodes from an event payload into the live graph. This is how
workflows generate workflows.

## Basic usage

```yaml
- id: planner
  driver: python
  handler: myapp:plan_tasks
  on: [start]
  emits: [plan.ready]

- id: absorb
  driver: runner
  on: [plan.ready]
  emits: [absorb.done]
```

The planner handler returns a payload like `{"nodes": [...]}`. The `runner`
driver takes that payload and injects the node specs into the graph.

## Emitted events

- The `runner` driver emits the first declared `emits` value when all nodes
  have been injected.
- Injected nodes usually subscribe to a later event (for example `absorb.done`)
  to start.

## Common `params:` fields

None. The `runner` driver only needs the `nodes` list from the trigger
event payload.

## See also

- [Self-modifying workflows](../../explanation/self-modifying.md)
- [Dynamically inject nodes](../../how-to/dynamically-inject-nodes.md)
