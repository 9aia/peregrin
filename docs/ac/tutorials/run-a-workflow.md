# Run a workflow

## Basic run

```bash
ac run workflow.yaml
```

## CLI flags

```bash
ac run workflow.yaml \
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
ac run workflow.yaml --dry-run
```

## Graceful shutdown

SIGINT and SIGTERM await in-flight tasks, clean up webhook servers, close the
store and DLQ, and emit `workflow.cancelled`.

```bash
ac run workflow.yaml --shutdown-timeout 30
```

## Concurrency control

Limit how many nodes run simultaneously. The default is 50.

```bash
ac run workflow.yaml --max-concurrency 5
```

## Dead-letter queue (DLQ)

Failed events are persisted by default to `.ac/<name>.dlq.jsonl`.

```bash
ac run workflow.yaml --dlq /var/log/ac/dlq.jsonl
ac run workflow.yaml --no-dlq
ac run workflow.yaml --export-dlq /tmp/dlq.json
```

## Persistence (default — no config needed)

Events are saved to `.ac/<workflow-name>.jsonl` automatically. Survives crashes.

```bash
ac run workflow.yaml
# events saved to .ac/<workflow-name>.jsonl
```

Opt out:

```bash
ac run workflow.yaml --no-store
```

## Replay (crash recovery)

Resume from a saved event log. `--replay` accepts JSON arrays and JSON lines.

```bash
ac run workflow.yaml --export-log /tmp/log.json
ac run workflow.yaml --replay /tmp/log.json
ac run workflow.yaml --replay .ac/my-workflow.jsonl
```

## Resume (failure recovery with retry)

`--resume` loads the default event log, prunes failure events, resets fire
counts for failed nodes, and preserves shared state.

```bash
ac run workflow.yaml
ac run workflow.yaml --resume
```

## Observability

Enable OpenTelemetry tracing:

```bash
ac run workflow.yaml --otel
```

Export to a collector:

```bash
ac run workflow.yaml --otel-endpoint http://collector:4317
ac run workflow.yaml --otel-endpoint http://collector:4318/v1/traces --otel-protocol http
```

## Prometheus metrics

```bash
ac run workflow.yaml --metrics 9090
```

Metrics at `http://127.0.0.1:9090/metrics`. Health check at `/health`.

## Secrets and environment

```bash
ac run workflow.yaml --secrets secrets.json
ac run workflow.yaml --env .env
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
