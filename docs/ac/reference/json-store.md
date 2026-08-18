# JSON store

`JsonStore` is the default `EventStore`. It persists every event as one line
of JSON to an append-only file. Survives crashes by default.

## Default path

```bash
ac run workflow.yaml
# → events saved to .ac/<workflow-name>.jsonl
```

## Override the path

```bash
ac run workflow.yaml --store /tmp/events.jsonl
```

```python
from agent_campaign.stores.json import JsonStore
runner = Runner(wf, store=JsonStore("/tmp/events.jsonl"))
```

## Behaviour

- Each event is one JSON line.
- The file is append-only.
- File permissions are `0600`.
- Use `--replay` to resume from a saved log.

## See also

- [Stores](../explanation/stores.md)
- [Memory store](memory-store.md)
- [Write a custom store](../how-to/write-a-custom-store.md)
