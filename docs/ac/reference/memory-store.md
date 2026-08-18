# Memory store

`MemoryStore` is an in-memory `EventStore`. Fast, no persistence.

## CLI

```bash
ac run workflow.yaml --no-store
```

## Programmatic

```python
from agent_campaign.stores.memory import MemoryStore
runner = Runner(wf, store=MemoryStore())
```

## When to use

- Tests that do not need crash recovery.
- Ephemeral runs where you do not want to leave state on disk.
- Benchmarks that want to exclude disk I/O.

## See also

- [Stores](../explanation/stores.md)
- [JSON store](json-store.md)
