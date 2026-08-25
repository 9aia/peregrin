# Run a workflow

## Basic run

```bash
handl run workflow.yaml
```

## CLI flags

```bash
handl run workflow.yaml \
  --log-level DEBUG \
  --log-format json \
  --secrets secrets.json \
  --env .env \
  --export-log /tmp/log.json \
  --export-dlq /tmp/dlq.json \
  --store /tmp/events.jsonl \
  --no-store \
  --dlq /tmp/dlq.jsonl \
  --no-dlq \
  --timeout 60 \
  --max-events 1000 \
  --max-concurrency 50 \
  --shutdown-timeout 10 \
  --no-cycle-detection \
  --cycle-window 20 \
  --cycle-threshold 3 \
  --otel \
  --otel-endpoint http://... \
  --otel-protocol grpc \
  --otel-service-name my-wf \
  --otel-payloads \
  --metrics 9090 \
  --replay /tmp/log.json \
  --resume \
  --dry-run
```

## Dry-run

Simulate the workflow without side effects. Shell nodes print their commands
instead of executing them. HTTP nodes print their requests. Webhook nodes print
their server address. The full event trace is still produced.

```bash
handl run workflow.yaml --dry-run
```

## Graceful shutdown

SIGINT and SIGTERM await in-flight tasks, clean up webhook servers, close the
store and DLQ, and emit `workflow.cancelled`.

```bash
handl run workflow.yaml --shutdown-timeout 30
```

## Concurrency control

Limit how many nodes run simultaneously. The default is 50.

```bash
handl run workflow.yaml --max-concurrency 5
```

## Dead-letter queue (DLQ)

Failed events are persisted by default to `.handl/<name>.dlq.jsonl`.

```bash
handl run workflow.yaml --dlq /var/log/handl/dlq.jsonl
handl run workflow.yaml --no-dlq
handl run workflow.yaml --export-dlq /tmp/dlq.json
```

## Persistence (default — no config needed)

Events are saved to `.handl/<workflow-name>.jsonl` automatically. Survives crashes.

```bash
handl run workflow.yaml
# events saved to .handl/<workflow-name>.jsonl
```

Opt out:

```bash
handl run workflow.yaml --no-store
```

## Replay (crash recovery)

Resume from a saved event log. `--replay` accepts JSON arrays and JSON lines.

```bash
handl run workflow.yaml --export-log /tmp/log.json
handl run workflow.yaml --replay /tmp/log.json
handl run workflow.yaml --replay .handl/my-workflow.jsonl
```

## Resume (failure recovery with retry)

`--resume` loads the default event log, prunes failure events, resets fire
counts for failed nodes, and preserves shared state.

```bash
handl run workflow.yaml
handl run workflow.yaml --resume
```

## Observability

Enable OpenTelemetry tracing:

```bash
handl run workflow.yaml --otel
```

Export to a collector:

```bash
handl run workflow.yaml --otel-endpoint http://collector:4317
handl run workflow.yaml --otel-endpoint http://collector:4318/v1/traces --otel-protocol http
```

## Prometheus metrics

```bash
handl run workflow.yaml --metrics 9090
```

Metrics at `http://127.0.0.1:9090/metrics`. Health check at `/health`.

## Secrets and environment

```bash
handl run workflow.yaml --secrets secrets.json
handl run workflow.yaml --env .env
```

### Public config: `AC_PUBLIC_*`

Use the `AC_PUBLIC_` prefix to route values to `config` instead of `secrets`:

```bash
AC_PUBLIC_REGION=us-east-1
```

Priority (later wins):

1. `--secrets` JSON file
2. `.env` in workflow directory (auto)
3. `--env` explicit path
4. Process environment variables

### Secret redaction

Secret values are redacted in CLI output, `--export-log`, and dry-run output.
They are not redacted in the event bus, the persistent store, or OTel spans.
