# Custom drivers and handlers

## Python handlers

Register a function that takes `(triggers, ctx, node)` and returns a list
of events:

```python
from agent_campaign.model import Event
from agent_campaign.drivers.python import register

def my_handler(triggers, ctx, node):
    e = triggers[0]
    return [Event(type="done", payload={"result": "ok"}, source=node.id)]

register("my_handler", my_handler)
```

Use in a workflow:

```yaml
- id: custom
  driver: python
  handler: my_handler
  on: [start]
  emits: [done]
```

Or use `module:func` notation without registering:

```yaml
- id: custom
  driver: python
  handler: myapp.handlers:my_handler
  on: [start]
  emits: [done]
```

## Importing Python modules

Use `import:` in the workflow YAML to import Python modules so their
handlers become available.

```yaml
import: [lib/litellm/handlers.py]

nodes:
  - id: review
    driver: python
    handler: llm:complete
    on: [pr.opened]
    emits: [review.done]
```

With an `as:` alias:

```yaml
import:
  - file: lib/litellm/handlers.py
    as: llm

nodes:
  - id: review
    driver: python
    handler: llm:complete
```

Dotted module paths (for installed packages):

```yaml
import: [myapp.handlers]
```

`import:` (Python modules) is distinct from `include:` (YAML file merging).

## Handler context (ctx)

```python
def my_handler(triggers, ctx, node):
    region = ctx.config.get("region", "us-east-1")
    api_key = ctx.get_secret("API_KEY")

    for e in ctx.log:
        print(e.type)

    ctx.emit("dynamic.event", {"data": 42})

    ctx.inject([
        {"id": "dynamic-1", "driver": "shell", "on": ["start"],
         "emits": ["dynamic-1.done"], "with": {"cmd": "echo hi"}},
    ])

    return [Event(type="done", source=node.id)]
```

## Shell driver conventions

The shell driver is the universal tool — most CLI agents are just shell
commands:

```yaml
- id: review
  driver: shell
  on: [pr.opened]
  emits: [review.done, review.failed]
  with:
    cmd: "claude --print 'Review this PR: ${event.payload.url}'"
```

### Exit codes → events

- `emits: [A, B]` → rc==0 emits `A`, rc!=0 emits `B`
- `with.emit: {0: A, 1: B, default: C}` → explicit mapping

### Environment + secrets + timeout

```yaml
with:
  cmd: "deploy.sh"
  env:
    REGION: us-east-1
  secrets: [API_KEY, DB_PASSWORD]
  timeout: 300
```

## Custom drivers

Implement the `Driver` protocol and register it:

```python
from agent_campaign.model import Context, Event, Node

class MyDriver:
    async def invoke(self, node: Node, triggers: list[Event], ctx: Context) -> list[Event]:
        return [Event(type=f"{node.id}.done", source=node.id)]

runner = Runner(wf, drivers={"my-driver": MyDriver()})
```

Use in a workflow:

```yaml
- id: custom
  driver: my-driver
  on: [start]
  emits: [custom.done]
```

## Agent drivers

Most CLI agents (claude-code, cursor, devin) are just shell commands. Use
the shell driver directly, or register a thin custom driver for richer APIs.

For `agent:*` driver names, the runner resolves the prefix — register a
driver under `"agent"` and it handles `agent:claude`, `agent:cursor`, etc.

## See also

- [Write a custom driver](write-a-custom-driver.md)
- [Self-modifying workflows](../explanation/self-modifying.md) — `ctx.inject()`
- [Use the programmatic API](programmatic-api.md)
