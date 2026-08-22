# ADR-0028: Provider-based observability CLI

## Status

Proposed

## Context

The runner emits three kinds of runtime output that the CLI currently exposes through ad-hoc flags:

- **Durable workflow record** — node results, events, span records, errors. ADR-0002 called this `ExportLog`; ADR-0027 called it `Traces`.
- **Live spans / traces** — currently exposed as `--otel`, `--otel-endpoint`, `--otel-protocol`, `--otel-service-name`.
- **Metrics** — currently exposed as `--metrics` and `--metrics-port`.

Each signal has its own custom flag set. The CLI hard-codes the provider (OTLP for traces, Prometheus for metrics, JSONL for export logs). There is no common pattern for selecting an implementation. A user who wants to send traces to a file, metrics to stdout, or history to S3 must use different syntax for each.

Industry tooling treats these as three separate signals with swappable backends:

- **History** — workflow execution history (AWS Step Functions, Temporal).
- **Tracing** — live spans and traces (OpenTelemetry).
- **Metrics** — counters and gauges (Prometheus, OpenTelemetry).

A provider abstraction keeps the CLI consistent and makes file-based, no-op, and cloud backends first-class.

## Decision

Expose three CLI signals — `--history`, `--tracing`, and `--metrics` — each with a provider abstraction and a file-based implementation by default.

1. Each signal uses two flags:
   - `--<signal>` — the destination string, interpreted by the provider.
   - `--<signal>-provider` — the implementation to use.
2. The `provider:dest` shorthand is also accepted: `--tracing otlp:http://collector:4318/v1/traces`.
3. Default providers:
   - `history`: `jsonl` (local file) when the flag is used.
   - `tracing`: `noop` when the flag is omitted; no default destination otherwise.
   - `metrics`: `noop` when the flag is omitted; no default destination otherwise.
4. Ports:
   - **History** — a new port in `ac-core` for durable workflow records.
   - **Observer** — the existing port for real-time spans; `--tracing` selects an `Observer` provider.
   - **Metrics** — the existing `Observer` also covers metrics; `--metrics` selects an `Observer` provider. When both `--tracing` and `--metrics` are set, the CLI composes a `CompositeObserver`.
5. Deprecated CLI flags:
   - `--otel` and its variants become `--tracing`.
   - `--metrics` and `--metrics-port` become `--metrics`.
   - `--export-log` / `--traces` become `--history`.

### CLI examples

```bash
# history
ac run workflow.yaml
# default: no history written

ac run workflow.yaml --history /tmp/run.jsonl
# provider defaults to jsonl

ac run workflow.yaml --history-provider s3 --history s3://bucket/prefix

ac run workflow.yaml --history otlp:http://collector:4318/v1/traces

# tracing
ac run workflow.yaml
# default: noop

ac run workflow.yaml --tracing otlp:http://collector:4318/v1/traces

ac run workflow.yaml --tracing-provider jsonl --tracing /tmp/spans.jsonl

# metrics
ac run workflow.yaml
# default: noop

ac run workflow.yaml --metrics prometheus:9090

ac run workflow.yaml --metrics-provider jsonl --metrics /tmp/metrics.jsonl
```

### Combined run

```bash
ac run workflow.yaml \
  --history /tmp/run.jsonl \
  --tracing otlp:http://collector:4318/v1/traces \
  --metrics prometheus:9090
```

### Core interfaces

```ts
export interface History {
  readonly append: (record: HistoryRecord) => Effect.Effect<void, never>;
  readonly close: () => Effect.Effect<void, never>;
}

export const History = Context.GenericTag<History>("ac/History");
```

`Observer` already exists. A `CompositeObserver` delegates to a tracing and a metrics `Observer` when both are configured.

```ts
export type HistoryRecord =
  | NodeRecord
  | EventRecord
  | SpanRecord
  | ErrorRecord;
```

### Default provider packages

| Signal | Package | Description |
|---|---|---|
| `history` | `ac-history-jsonl` | One JSON record per line, append-only. |
| `history` | `ac-history-s3` | Batches to S3 as `.jsonl` or `.parquet` (future). |
| `history` | `ac-history-otlp` | Forwards records to an OTLP collector (future). |
| `tracing` | `ac-observer-noop` | No output. |
| `tracing` | `ac-observer-otel` | OTLP spans. |
| `tracing` | `ac-observer-jsonl` | Writes spans to a file (future). |
| `metrics` | `ac-observer-noop` | No output. |
| `metrics` | `ac-observer-prometheus` | Prometheus `/metrics` endpoint. |
| `metrics` | `ac-observer-jsonl` | Writes metrics to a file (future). |

## Consequences

- The CLI becomes consistent across history, tracing, and metrics.
- New observability backends can be added as packages without touching the CLI parser.
- Existing flags (`--otel`, `--metrics`, `--export-log`, `--traces`) must be migrated or deprecated.
- Documentation and examples must be updated to the new `--history`, `--tracing`, `--metrics` flags.
- The `History` port must be added to `ac-core` and wired into the runner.
- `CompositeObserver` or an equivalent is needed to support `--tracing` and `--metrics` at the same time.
- File-based defaults make it easy to replay or inspect runs without external infrastructure.

## Alternatives considered

- **Keep hard-coded flags per backend.** Simpler today, but becomes unmanageable as more backends are added (S3, Kafka, Datadog, etc.).
- **Use a single `--observability` flag for all three signals.** Too coarse; users often want different destinations for traces and metrics.
- **Reuse only `--tracing` and `--metrics` and drop `History` into `EventStore`.** `EventStore` is replay state, not an exportable audit record. Keeping them separate preserves the right boundaries.
