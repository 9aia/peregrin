# Observers (observability)

Lifecycle hooks flow through an `Observer` adapter. The runner calls the
observer at key points: workflow start/end, node start/end, event emission,
node injection, and node retirement. Swap implementations without touching
the runner.

## Built-in observers

- `NoopObserver` (default) — silent, zero overhead.
- `OTelObserver` — OpenTelemetry spans and metrics.

The observer is an adapter port: the runner does not know which
implementation you use, so observability can be tailored to the deployment.

## Why an adapter port?

Observability is deployment-specific. A local dev run does not need tracing.
A production run needs OTel or Datadog. A debug session might just log.
The adapter port lets you swap without touching the runner.

The `Span` returned by `on_node_start` is a context manager closed by
`on_node_end`. This decouples span lifecycle from the runner — the observer
decides whether to create a real trace span or a no-op.

## See also

- [OpenTelemetry observer](../reference/otel-observer.md)
- [Write a custom observer](../how-to/write-a-custom-observer.md)
- [Adapter ports](adapter-ports.md) — the port architecture
