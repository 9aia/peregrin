# Dynamically inject nodes

There are two ways to inject nodes at runtime: through the `runner` driver
or directly from a Python handler.

## Using the `runner` driver

A Python handler emits an event whose payload contains `nodes: [...]`. The
`runner` driver picks up that event and injects the specs.

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

```python
def plan_tasks(triggers, ctx, node):
    tasks = node.with_.get("tasks", [])
    new_nodes = []
    for i, task in enumerate(tasks):
        new_nodes.append({
            "id": f"dynamic-{i}",
            "driver": "shell",
            "on": ["absorb.done"],
            "emits": [f"dynamic-{i}.done"],
            "with": {"cmd": task["cmd"]},
        })
    return [Event(type="plan.ready", payload={"nodes": new_nodes}, source=node.id)]
```

## Using `ctx.inject()`

A Python handler can inject nodes directly without an intermediate event:

```python
def my_handler(triggers, ctx, node):
    ctx.inject([
        {"id": "dynamic-1", "driver": "shell", "on": ["start"],
         "emits": ["dynamic-1.done"], "with": {"cmd": "echo hi"}},
    ])
    return [Event(type="injected", source=node.id)]
```

## Scope injected nodes with `retire_on`

Injected nodes live until the workflow ends unless you remove them. Use
`retire_on` to clean up:

```yaml
- id: temp-worker
  on: [task.assigned]
  emits: [task.done]
  retire_on: [task.done]
```

## See also

- [Self-modifying workflows](../explanation/self-modifying.md) — concept
- [Loop protection](../explanation/loop-protection.md) — `max_fires`, `until`, `retire_on`
- [runner driver reference](../reference/drivers/runner.md)
