# Dead-letter queue (DLQ)

When a node fails and has no retry (or retries are exhausted), the runner emits a `dlq.event` event so downstream nodes can react.

## DLQ event payload

```yaml
type: dlq.event
source: __runner__
payload:
  node: <node-id>
  error: <error-message>
  event: node.failed
```

## Reacting to DLQ events

```yaml
- id: alert-dlq
  driver: shell
  on: [dlq.event]
  emits: [alerted]
  with:
    cmd: "echo 'DLQ: node ${event.payload.node} failed: ${event.payload.error}'"
```

## Retries

A node can retry before the DLQ event is emitted:

```yaml
- id: flaky
  driver: http
  on: [start]
  emits: [flaky.done, flaky.failed]
  retry:
    max_attempts: 3
    delay: 1
    backoff: exponential
```

After the configured `max_attempts` are exhausted, the runner emits `dlq.event` and `node.failed`.

## See also

- [Resilience](../explanation/resilience.md)
- [Combine resilience mechanisms](../how-to/combine-resilience-mechanisms.md)
