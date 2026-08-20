# Retry

Retry failed nodes automatically with configurable backoff.

## Configuration

```yaml
- id: call-api
  driver: http
  on: [start]
  emits: [api.done, api.failed]
  retry:
    max_attempts: 3
    backoff: exponential
    delay: 1
  params:
    method: GET
    url: "https://api.example.com/data"
```

## Fields

| Field          | Type   | Description                         |
| -------------- | ------ | ----------------------------------- |
| `max_attempts` | int    | Maximum attempts                    |
| `backoff`      | string | `fixed`, `linear`, or `exponential` |
| `delay`        | number | Base delay in seconds               |

## Backoff strategies

| Strategy      | Formula                 | Example (delay=1) |
| ------------- | ----------------------- | ----------------- |
| `fixed`       | `delay`                 | 1s, 1s, 1s        |
| `linear`      | `delay * attempt`       | 1s, 2s, 3s        |
| `exponential` | `delay * 2^(attempt-1)` | 1s, 2s, 4s        |

## Behaviour

When a node with `retry` config fails:

1. Wait `delay` seconds (adjusted by backoff).
2. Re-run the node.
3. Repeat until `max_attempts` is reached.
4. If all attempts fail, the event goes to the DLQ.

## See also

- [Resilience](../explanation/resilience.md)
- [Combine resilience mechanisms](../how-to/combine-resilience-mechanisms.md)
