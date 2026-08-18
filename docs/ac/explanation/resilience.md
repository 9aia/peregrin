# Resilience: Retry, DLQ, Circuit Breaker

Production workflows need failure handling. Three mechanisms work together:

1. **Retry** — try a failed node again with backoff.
2. **Dead-letter queue (DLQ)** — persist failures that cannot be recovered.
3. **Circuit breaker** — stop hitting a failing node after repeated failures.

## Retry

Retry failed nodes automatically with configurable backoff.

```yaml
- id: call-api
  driver: http
  on: [start]
  emits: [api.done, api.failed]
  retry:
    max_attempts: 3
    backoff: exponential
    delay: 1
  with:
    method: GET
    url: "https://api.example.com/data"
```

## Dead-Letter Queue

When a node fails and has no retry (or retries are exhausted), the failure
event goes to the DLQ. The runner emits a `dlq.event` event so downstream
nodes can react.

## Circuit breaker

Stop hitting a failing node after N consecutive failures. Gives downstream
services time to recover.

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

## Resource management

Resource exhaustion is different from exceptions: heavy handlers spawn OS
processes that can thrash the machine. The engine adds backpressure:

- `--max-concurrency` bounds how many nodes run at once.
- `--max-cpu-percent`, `--max-mem-percent`, `--min-free-mem-mb` defer nodes
  until the system can handle them.
- `resources.weight` lets a node count as more than one slot.
- A global load circuit breaker trips on sustained overload and emits
  `system.overloaded`.
- `mem_limit_mb` and `oom_score_adj` hard-cap child processes.

## Why these three?

- **Retry** handles transient problems.
- **DLQ** quarantines persistent failures.
- **Circuit breaker** protects downstream services from overload.
- **Resource management** protects the orchestrator itself.

## See also

- [Retry](../reference/retry.md)
- [Circuit breaker](../reference/circuit-breaker.md)
- [Dead-letter queue](../reference/dead-letter-queue.md)
- [Resource management](../reference/resource-management.md)
- [Combine resilience mechanisms](../how-to/combine-resilience-mechanisms.md)
