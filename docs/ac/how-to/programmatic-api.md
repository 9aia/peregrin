# Use the programmatic API

Everything the CLI does, you can do in Python.

## Basic run

```python
import asyncio
from agent_campaign import Workflow, Node
from agent_campaign.runner import Runner

wf = Workflow(nodes=[
    Node(id="hello", driver="shell", on=["start"],
         emits=["hello.done"],
         with_={"cmd": "echo 'hello'"}),
])

runner = Runner(wf)
log = asyncio.run(runner.run())
for e in log:
    print(f"  {e.type} [{e.source}]")
```

## Load from YAML

```python
runner = Runner.from_yaml("workflow.yaml")
log = asyncio.run(runner.run())
```

## Persistence (default: JsonStore)

```python
runner = Runner(wf)
asyncio.run(runner.run())
```

Override the path:

```python
from agent_campaign.stores.json import JsonStore
runner = Runner(wf, store=JsonStore("/tmp/events.jsonl"))
```

Opt out:

```python
from agent_campaign.stores.memory import MemoryStore
runner = Runner(wf, store=MemoryStore())
```

## Replay (resume)

```python
from agent_campaign.model import Event

saved = [
    Event(type="start", source="__runner__"),
    Event(type="a.done", source="a"),
]

runner = Runner(wf)
log = runner.replay(saved)
```

## Observability

```python
from agent_campaign.observers.otel import OTelObserver

runner = Runner(wf, observer=OTelObserver(service_name="my-service"))
asyncio.run(runner.run())
```

## Custom store or observer

```python
runner = Runner(
    wf,
    store=MyPostgresStore(),
    observer=MyDatadogObserver(),
)
```

## Loop protection

```python
runner = Runner(
    wf,
    max_events=1000,
    timeout=60,
    cycle_detection=True,
    cycle_window=20,
    cycle_threshold=3,
)
```

## Inspect the event log

```python
log = asyncio.run(runner.run())

for e in log:
    print(e.type, e.payload, e.source)

for e in runner.store.all():
    print(e.type)

for e in runner.store.by_type("lint.passed"):
    print(e.payload)

print(runner.store.seen_types())
```

## Export the log

```python
runner.export_log("/tmp/log.json")
```

## See also

- [Custom drivers and handlers](custom-drivers.md)
- [Run a workflow](../tutorials/run-a-workflow.md) — CLI flags reference
