# Handler context (ctx)

The `ctx` object gives handlers access to config, secrets, the event log,
and runtime actions.

```python
def my_handler(triggers, ctx, node):
    # Read workflow config
    region = ctx.config.get("region", "us-east-1")

    # Read secrets — preferred API
    api_key = ctx.get_secret("API_KEY")

    # Direct dict access also works
    api_key = ctx.secrets.get("API_KEY")

    # Idempotency key — unique per node firing
    key = ctx.idempotency_key

    # Read the full event log
    for e in ctx.log:
        print(e.type, e.payload)

    # Emit events dynamically (beyond declared emits)
    ctx.emit("dynamic.event", {"data": 42})

    # Inject new nodes at runtime
    ctx.inject([
        {"id": "dynamic-1", "driver": "shell", "on": ["start"],
         "emits": ["dynamic-1.done"], "with": {"cmd": "echo hi"}},
    ])

    # Fire counter for this node
    attempt = ctx.fires

    # Shared scratch state
    ctx.state["last_run"] = time.time()

    return [Event(type="done", source=node.id)]
```

## Attributes and methods

| Name                      | Description                           |
| ------------------------- | ------------------------------------- |
| `ctx.config`              | Workflow-level `config` dict          |
| `ctx.secrets`             | Dict of resolved secrets              |
| `ctx.get_secret(name)`    | Preferred API for reading a secret    |
| `ctx.log`                 | All events emitted so far             |
| `ctx.idempotency_key`     | Unique string per node firing         |
| `ctx.fires`               | 1-based attempt counter for this node |
| `ctx.state`               | Persisted key/value store             |
| `ctx.emit(type, payload)` | Emit an event directly                |
| `ctx.inject(node_specs)`  | Inject new nodes into the graph       |

## See also

- [Handlers](../explanation/handlers.md)
- [Dynamically inject nodes](../how-to/dynamically-inject-nodes.md)
