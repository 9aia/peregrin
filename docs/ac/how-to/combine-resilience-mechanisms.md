# Combine resilience mechanisms

Retry, DLQ, and circuit breaker can be configured on the same node. This is
useful for calls to flaky external services.

```yaml
nodes:
  - id: call-api
    driver: http
    on: [start]
    emits: [api.done, api.failed]
    retry:
      max_attempts: 3
      backoff: exponential
      delay: 1
    circuit_breaker:
      threshold: 10
      reset_after: 300
      on_trip: api.circuit_open
    with:
      method: GET
      url: "https://api.example.com/data"

  - id: alert
    driver: shell
    on: [api.circuit_open]
    emits: [alerted]
    with:
      cmd: "echo 'API circuit breaker opened'"

  - id: dlq-alert
    driver: shell
    on: [dlq.event]
    emits: [dlq.alerted]
    with:
      cmd: "echo 'Event sent to DLQ: ${event.payload.node}'"
```

## How it works

1. The HTTP call is attempted up to 3 times with exponential backoff.
2. If it still fails, the event goes to the DLQ and `dlq.event` is emitted.
3. After 10 consecutive failures, the circuit breaker opens and emits
   `api.circuit_open`. The node is skipped until `reset_after` seconds pass.

## Resource-aware example

Add resource hints so heavy retries do not drown the machine:

```yaml
- id: call-api
  driver: http
  on: [start]
  emits: [api.done, api.failed]
  resources: { weight: 2 }
  retry:
    max_attempts: 3
    backoff: exponential
    delay: 1
  with:
    method: GET
    url: "https://api.example.com/data"
```

## See also

- [Resilience](../explanation/resilience.md) — concepts
- [Retry](../reference/retry.md), [Circuit breaker](../reference/circuit-breaker.md), [Dead-letter queue](../reference/dead-letter-queue.md)
- [Resource management](../reference/resource-management.md)
