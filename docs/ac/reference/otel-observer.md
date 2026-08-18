# OpenTelemetry observer

`OTelObserver` emits OpenTelemetry spans for the workflow and every node.

## Console only

```bash
pip install opentelemetry-api opentelemetry-sdk
ac run workflow.yaml --otel
```

```python
from agent_campaign.observers.otel import OTelObserver

runner = Runner(wf, observer=OTelObserver(service_name="my-service"))
```

## OTLP export to a collector

```bash
pip install opentelemetry-exporter-otlp-proto-grpc
ac run workflow.yaml --otel-endpoint http://collector:4317 --otel-service-name my-wf
```

```python
runner = Runner(wf, observer=OTelObserver(
    service_name="my-service",
    console=False,
    otlp_endpoint="http://collector:4317",
    otlp_protocol="grpc",
))
```

## CLI flags

| Flag                  | Description                              |
| --------------------- | ---------------------------------------- |
| `--otel`              | Enable console tracing                   |
| `--otel-endpoint`     | OTLP collector URL                       |
| `--otel-protocol`     | `grpc` (default) or `http`               |
| `--otel-service-name` | Service name for spans                   |
| `--otel-payloads`     | Include redacted event payloads in spans |

## Distributed tracing (W3C traceparent)

When the `http` driver makes requests and `--otel` is enabled, a W3C
`traceparent` header is automatically injected.

## See also

- [Observers](../explanation/observers.md)
- [Write a custom observer](../how-to/write-a-custom-observer.md)
- [Run a workflow](../tutorials/run-a-workflow.md) — CLI observability flags
