# Dead-letter queue (DLQ)

When a node fails and has no retry (or retries are exhausted), the failure
event goes to the DLQ. The runner emits a `dlq.event` event so downstream
nodes can react.

## Default path

```
.ac/<name>.dlq.jsonl
```

## CLI

```bash
# Default: persistent DLQ
ac run workflow.yaml

# Override path
ac run workflow.yaml --dlq /var/log/ac/dlq.jsonl

# Disable persistence
ac run workflow.yaml --no-dlq

# Export DLQ as JSON after run
ac run workflow.yaml --export-dlq /tmp/dlq.json
```

## Programmatic access

```python
runner = Runner(wf)
asyncio.run(runner.run())
print(f"DLQ: {len(runner.dlq)} events")
for e in runner.dlq:
    print(f"  {e.type} from {e.payload['node']}: {e.payload['error']}")
```

The DLQ is cleared at the start of each run.

## Reacting to DLQ events

```yaml
- id: alert-dlq
  driver: shell
  on: [dlq.event]
  emits: [alerted]
  with:
    cmd: "echo 'DLQ: node ${event.payload.node} failed: ${event.payload.error}'"
```

## See also

- [Resilience](../explanation/resilience.md)
- [Combine resilience mechanisms](../how-to/combine-resilience-mechanisms.md)
