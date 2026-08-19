# Drivers

Drivers execute handlers. A node declares `driver: <name>`, and the runner
looks up the driver in its registry. The driver receives the node, trigger
events, and context — it runs the handler and returns emitted events.

## Built-in drivers

- `shell` — runs a shell command. Exit codes map to events.
- `python` — calls a Python function by name (`handler: my_func` or `handler: module:func`).
- `runner` — injects new nodes from an event payload into the live graph.
- `http` — makes HTTP requests. Status codes map to events.
- `webhook` — starts an HTTP server. POSTs emit events. Keeps the workflow alive.
- `stdin` — reads lines from stdin. Emits one event per line. Keeps the workflow alive.
- `cli` — polls a shell command on an interval. Emits one event per output line.
- `agent` — runs an external AI coding agent CLI and maps exit codes to events.
- `human` — prompts a human on stdin and maps the response to events.

## The driver registry

The runner maintains a registry that maps driver names to `DriverProtocol`
implementations. Built-in drivers are registered by default. You can also
pass a custom dictionary:

```python
runner = Runner(wf, drivers={"http": HTTPDriver()})
```

A driver only needs a single `invoke` method.

## Agent prefix convention

For `agent:*` driver names, the runner resolves the prefix. Register a
driver under `"agent"` and it handles `agent:claude`, `agent:cursor`, etc.:

```python
class AgentDriver:
    async def invoke(self, node, triggers, ctx):
        agent_name = node.driver.split(":")[1]
        # ... run and return events

runner = Runner(wf, drivers={"agent": AgentDriver()})
```

```yaml
- id: review
  driver: agent:claude
  on: [pr.opened]
  emits: [review.done]
  with:
    prompt: "Review this PR: ${event.payload.url}"
```

The `agent:*` scheme lets a single driver support many agent CLIs while the
workflow author controls which one runs.

## See also

- [Write a custom driver](../how-to/write-a-custom-driver.md)
- Driver reference: [shell](../reference/drivers/shell.md), [python](../reference/drivers/python.md), [runner](../reference/drivers/runner.md), [http](../reference/drivers/http.md), [webhook](../reference/drivers/webhook.md), [stdin](../reference/drivers/stdin.md), [cli](../reference/drivers/cli.md), [agent](../reference/drivers/agent.md), [human](../reference/drivers/human.md)
- [Streaming](../reference/drivers/streaming.md) and [polling](../reference/drivers/polling.md)
