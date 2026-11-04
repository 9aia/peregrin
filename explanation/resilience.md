# Resilience: retry, DLQ, circuit breaker

Production workflows need failure handling. Three mechanisms work
together — all declared on the node, all enforced by the runner:

1. **Retry** — try a failed node again with backoff.
2. **Dead-letter queue** — keep failures that cannot be recovered.
3. **Circuit breaker** — stop hitting a failing node after repeated
   failures.

## Retry

Retry failed nodes automatically with configurable backoff.

```yaml
- id: call-api
  driver: http
  on: [start]
  emits: [api.done, api.failed]
  retry:
    max_attempts: 3        # default 1
    delay: 1               # seconds, default 1
    backoff: exponential   # fixed | linear | exponential
  params:
    method: GET
    url: "https://api.example.com/data"
```

Attempts run inline — the sequential runner sleeps the delay between
them. Success resets the attempt counter and clears the breaker
half-lap.

## Dead-letter queue

When a node fails with no retry left (or none configured), the
runner emits `node.failed` and enqueues it on the
`DeadLetterQueue` port — `MemoryDlq` under the stock node
composition keeps dead-lettered events for the life of the run;
`NoopDlq` drops them. A durable DLQ is one `with_dlq` swap away.

## Circuit breaker

Stop hitting a failing node after N consecutive failures. Gives
downstream services time to recover.

```yaml
- id: call-external-api
  driver: http
  on: [start]
  emits: [api.done, api.failed]
  circuit_breaker:
    threshold: 5          # consecutive failures to trip (default 5)
    reset_after: 60       # seconds before half-open (default 60)
    on_trip: circuit.open # event type to emit on trip
  params:
    method: GET
    url: "https://flaky-api.example.com/data"
```

While the breaker is open the node short-circuits to
`node.failed` ("circuit open") without invoking the driver; the
configured `on_trip` type (default `circuit.open`) is emitted once
per trip so the graph can react — a subscriber can page, degrade,
or `stop_on` it.

## Resource bounds — mostly not ported

v1's resource-management layer gated nodes on machine load —
`--max-concurrency`, CPU/memory probes, a load circuit breaker
(`system.overloaded`), `mem_limit_mb`/`oom_score_adj`. Of that, the
port keeps only the *shape*: `resources.weight` (min 1) feeds a
`ResourceGate` whose 50-slot cap can never be hit under the
sequential runner — the arithmetic is kept for parity, the OS-probe
halves are not.

The bounds that *do* constrain the machine today live in drivers:
hard timeout caps (shell 600 s, http/agent/human 300–600 s),
captured-output limits, the webhook 2 MiB body cap. See
[Loop protection](loop-protection.md) for the run-level picture.

## Why these three?

- **Retry** handles transient problems.
- **DLQ** quarantines persistent failures — as data, not just log
  lines.
- **Circuit breaker** protects downstream services from overload.
- **Timeouts and capture caps** protect the orchestrator itself.

Failures are contained, not cascading: a `node.failed` is just an
event — subscribers decide whether the run should degrade, retry,
or end.

## See also

- [Campaign schema](../reference/campaign-schema.md) — `retry`,
  `circuit_breaker`, `resources` fields
- [Adapter ports](adapter-ports.md) — the DLQ port
- [Loop protection](loop-protection.md) — the bounds on *correct*
  runs
