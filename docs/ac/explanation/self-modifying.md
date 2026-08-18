# Self-modifying workflows

A node emits an event whose payload _is_ new node specs. The `runner` driver
injects them into the live graph. This lets a workflow grow at runtime
instead of declaring every step in advance.

## Why self-modify?

Some workflows cannot know the full graph up front. A planning step may
discover how many tasks are needed. A refinement loop may add verification
nodes. A dispatch step may fan out to a variable number of workers.

## Basic injection

```yaml
nodes:
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

The planner handler returns node specs in the event payload. The `runner`
driver absorbs them and adds them to the graph. The injected nodes then
subscribe to `absorb.done` (or another event) to begin work.

## Scoping injected nodes (retire_on)

Injected nodes accumulate in the graph forever — unless they clean themselves
up with `retire_on`:

```yaml
- id: temp-worker
  on: [task.assigned]
  emits: [task.done]
  retire_on: [task.done]
```

Unlike `until` (which stops a node from firing but leaves it in the graph),
`retire_on` deletes the node entirely. This prevents unbounded growth in
self-modifying workflows that inject in a loop.

## See also

- [Dynamically inject nodes](../how-to/dynamically-inject-nodes.md)
- [Loop protection](loop-protection.md) — `max_fires`, `until`, `retire_on`
- [runner driver reference](../reference/drivers/runner.md)
