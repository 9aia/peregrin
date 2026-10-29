+++
id = "ADR-0065"
title = "ADR-0027: Rename export-log to traces"
status = "open"
priority = "medium"
+++

# ADR-0027: Rename export-log to traces

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Superseded by ADR-0028.

## Context

ADR-0002 introduced `ExportLog` as a first-class port for durable records of node results, span events, and failed events destined for external systems. Industry tools do not use the term "export log". The records the port carries — node results, span events, and errors — are most commonly called "traces" in observability and workflow systems (OpenTelemetry, Temporal, AWS Step Functions). Using a non-standard name makes the port harder to discover and reason about.

## Decision

Rename the `ExportLog` port and related types to `Traces`.

1. The port `ExportLog` becomes `Traces`.
2. The `ExportRecord` union becomes `TraceRecord`.
3. The default implementation `handl-export-jsonl` becomes `handl-traces-jsonl`.
4. The CLI/config option `exportLog` / `--export-log` becomes `traces` / `--traces` as a destination string.
5. Add a `--traces-provider` option to select the implementation. The default provider is `jsonl`.
6. The default local file becomes `.traces.jsonl`.

### Format and provider abstraction

The `Traces` port itself is format-agnostic. The implementation owns the serialization:

- `handl-traces-jsonl` (default) — one JSON record per line, append-only, matching the `EventStore` `JsonStore` pattern.
- `handl-traces-stdout` — human-readable or NDJSON to `stdout`.
- `handl-traces-otlp` — forwards records to an OTLP collector, distinct from the live `--otel` observer.
- `handl-traces-s3` — writes batches to S3 as `.jsonl` or `.parquet`.

The CLI value is interpreted by the active provider. For the default `jsonl` provider it is a file path:

```bash
handl run workflow.yaml --traces /tmp/traces.jsonl
```

For other providers the value can be a URI or endpoint:

```bash
handl run workflow.yaml --traces-provider otlp --traces http://collector:4318/v1/traces
handl run workflow.yaml --traces-provider s3 --traces s3://bucket/prefix
```

This keeps the CLI flag stable while allowing new formats without changing the port interface.

### Relationship to existing CLI observability flags

| Flag / port | What it does | Output | When to use |
|---|---|---|---|
| `--log-level` (docs only) | Controls verbosity of the CLI's own logging. | `stderr` | Debug the CLI itself. |
| `--log-format` | Format of the CLI's own output and `MetricsObserver` rendering. | `stdout` / Prometheus | Read logs or metrics in `text` or `json`. |
| `--otel` + `--otel-endpoint` | Enables the OpenTelemetry `Observer`, sending live spans during the run. | OTLP collector (grpc/http) | Real-time distributed tracing. |
| `--metrics` + `--metrics-port` | Starts a Prometheus `/metrics` and `/health` server. | HTTP endpoint | Scraped counters and gauges. |
| `--export-log` / `--traces` | Durable `Traces` port: node results, events, span records, errors. | File, S3, OTLP, stdout | Replay, audit, or batch export after the run. |
| `--replay` | Reads a previously exported event/traces log to replay or resume. | File input | Recover from crash or test a run. |

`--otel` and `--traces-provider otlp` can point at the same collector, but they serve different purposes: `--otel` sends live spans as the workflow executes; `Traces` appends durable workflow records that can be forwarded after the fact. The two are not replacements for each other.

### Interface after the change

```ts
export interface Traces {
  readonly append: (record: TraceRecord) => Effect.Effect<void, never>;
  readonly close: () => Effect.Effect<void, never>;
}

export const Traces = Context.GenericTag<Traces>("handl/Traces");
```

### Record shape

```ts
export type TraceRecord =
  | NodeRecord
  | EventRecord
  | SpanRecord
  | ErrorRecord;
```

## Consequences

- The name matches the dominant terminology in observability and workflow tooling.
- Existing references to `ExportLog`, `ExportRecord`, `handl-export-jsonl`, and `.export-log.jsonl` must be renamed.
- ADR-0002 remains the source for the DLQ and queue ports; its `ExportLog` section is superseded by this ADR.
- Documentation and examples must use `traces` and `.traces.jsonl`.

## Alternatives considered

- `AuditLog` — accurate for compliance, but too narrow if the main consumer is observability.
- `Telemetry` — good, but "traces" is the more specific signal for span and node records.
- `TraceStore` — consistent with `EventStore` and `SecretStore`, but "traces" as a signal name is more idiomatic in the ecosystem and keeps the API closer to OpenTelemetry.
