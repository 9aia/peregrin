# Circuit breaker

Stop hitting a failing node after N consecutive failures. Prevents cascading
failures and gives the downstream service time to recover.

## Configuration

```yaml
- id: call-external-api
  driver: http
  on: [start]
  emits: [api.done, api.failed]
  circuit_breaker:
    threshold: 5
    reset_after: 60
    on_trip: circuit.open
  with:
    method: GET
    url: "https://flaky-api.example.com/data"
```

## Fields

| Field         | Type   | Description                                 |
| ------------- | ------ | ------------------------------------------- |
| `threshold`   | int    | Failures before the breaker opens           |
| `reset_after` | int    | Seconds before half-open (one test allowed) |
| `on_trip`     | string | Event to emit when the breaker opens        |

## Behaviour

When the circuit opens:

- The node is skipped on subsequent triggers.
- A `circuit.open` event (or whatever `on_trip` is set to) is emitted.
- After `reset_after` seconds, the breaker enters half-open state.

## Reacting to the trip

```yaml
- id: alert-circuit-open
  driver: shell
  on: [circuit.open]
  emits: [alerted]
  with:
    cmd: "pagerduty-alert 'Circuit open for ${event.payload.node}'"
```

## See also

- [Resilience](../explanation/resilience.md)
- [Combine resilience mechanisms](../how-to/combine-resilience-mechanisms.md)
