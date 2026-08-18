# Write a custom driver

Implement the `DriverProtocol` — one async method:

```python
from agent_campaign.model import Context, Event, Node

class HTTPDriver:
    async def invoke(
        self, node: Node, triggers: list[Event], ctx: Context
    ) -> list[Event]:
        import aiohttp
        cfg = node.with_
        async with aiohttp.ClientSession() as s:
            async with s.request(
                cfg.get("method", "GET"),
                cfg["url"],
                json=cfg.get("body"),
                headers=cfg.get("headers", {}),
            ) as r:
                payload = {"status": r.status, "body": await r.text()}
                event_type = node.emits[0] if r.status < 400 else node.emits[1]
                return [Event(type=event_type, payload=payload, source=node.id)]
```

Register it:

```python
runner = Runner(wf, drivers={"http": HTTPDriver()})
```

Use it:

```yaml
- id: webhook
  driver: http
  on: [deploy.ready]
  emits: [webhook.sent, webhook.failed]
  with:
    method: POST
    url: "https://api.slack.com/services/notify"
    body:
      text: "Deployment ready"
```

## The protocol

A driver is just an object with an `invoke` method. It receives the node
definition, the list of trigger events, and the runtime context. It must
return a list of `Event` objects. The driver decides what to do with
`node.with_` and how to map outcomes to the declared `emits`.

## Naming and registry

Driver names are arbitrary strings. Register them when constructing the
runner:

```python
runner = Runner(wf, drivers={"my-driver": MyDriver(), "agent": AgentDriver()})
```

A node uses the name directly:

```yaml
- id: custom
  driver: my-driver
```

For `agent:*` prefixes, register the driver under the base name and parse
the suffix inside `invoke`.

## See also

- [Drivers](../explanation/drivers.md) — the driver model
- [Custom drivers and handlers](custom-drivers.md) — Python handlers and the shell driver
