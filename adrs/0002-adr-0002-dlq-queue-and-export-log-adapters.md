+++
id = "ADR-0002"
title = "ADR-0002: DLQ, queue, and export-log adapters"
status = "open"
priority = "medium"
+++

# ADR-0002: DLQ, queue, and export-log adapters

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

The engine currently produces three kinds of persisted or observed output:

- **Event store** — the live log of events used for replay and state recovery. This is already a port: `handl-core` defines `EventStore`, and packages like `handl-store-memory` and `handl-store-json` provide implementations.
- **Dead-letter queue (DLQ)** — events that fail validation, driver errors, or retries and are not part of the normal workflow. Today the runner emits a hardcoded `dlq.event` event (see `packages/handl-engine/src/runner.ts`). There is no adapter; the only consumer is the event log.
- **Export log** — a stream of node results, span events, and failed events for external systems. Today the runner emits `Observer` callbacks, but there is no dedicated export-log adapter. Users who want to send run data to a file, S3, an SIEM, or an analytics tool must build it themselves inside an observer.

The store and observer abstractions already prove the pattern works. We should apply the same adapter pattern to DLQs and export logs. The default implementation can be a local `.jsonl` file, but the port must allow other backends (SQS, RabbitMQ, Kafka, S3, stdout, OpenTelemetry, etc.).

The question was also raised: is the **queue** itself an adapter? A Peregrin workflow does not need a message queue to run, but in production deployments it will often be fed by one (e.g. a webhook queue, a CI event bus). Treating inbound and outbound queues as adapters lets Peregrin integrate with existing infrastructure without becoming a queue implementation.

## Decision

Introduce three new ports in `handl-core` following the same pattern as `EventStore` and `Observer`:

1. **`DeadLetterQueue`**
   - `enqueue(event: Event, reason: string): void`
   - `all(): ReadonlyArray<Event>`
   - `close(): void`
   - The default implementation, `handl-dlq-jsonl`, appends to a local `.dlq.jsonl` file.
   - Future implementations can target SQS, RabbitMQ, Redis, Kafka, etc.

2. **`ExportLog`** — superseded by ADR-0028.
   - The naming and CLI integration moved to the provider-based history/traces/metrics design in ADR-0028.
   - The record shape (`NodeRecord`, `EventRecord`, `SpanRecord`, `ErrorRecord`) survives as the `HistoryRecord` in ADR-0028.

3. **`Queue` (optional inbound/outbound adapter)**
   - `subscribe(eventType: string, handler: (event: Event) => void): void`
   - `publish(event: Event): void`
   - `close(): void`
   - The `Queue` port is used only when the runner is embedded in an external message bus. It is not required for file-based CLI runs.
   - The default implementation, `handl-queue-memory`, is a no-op for single-process runs.

The runner will accept these as Effect layers, just like `EventStore` and `Observer`. The current `dlq.event` emissions will be replaced by `DeadLetterQueue.enqueue(...)` in `handl-engine/src/runner.ts`. The `Observer` port will continue to emit real-time spans and node events, but `ExportLog` will be the durable, line-based record for external systems.

## Consequences

- DLQ, export-log, and queue behaviour become first-class, testable, and swappable.
- New packages (`handl-dlq-jsonl`, `handl-export-jsonl`, `handl-queue-memory`) need to be created as defaults.
- `handl-engine/src/runner.ts` will depend on `DeadLetterQueue` and `ExportLog` in addition to `EventStore` and `Observer`.
- The `Runner` constructor will accept `dlqLayer` and `exportLogLayer` options, with sensible defaults.
- Existing `Observer` implementations that also want to export records can simply implement `ExportLog` or compose both.
- The `dlq.event` type can be deprecated or kept as a legacy `Observer` event, but the canonical DLQ is the `DeadLetterQueue` port.
- Future work can add `handl-dlq-sqs`, `handl-export-otel`, `handl-queue-kafka`, etc., without touching the engine.

## Alternatives considered

- **Keep `dlq.event` and `export` as observer events only.** This forces every consumer to re-derive the record shape from observer callbacks. It also conflates real-time tracing with durable export, which have different retention and delivery needs.
- **Use a single `Log` port for everything.** A single `Log` port is simpler but loses the semantic difference between the event store (replayable workflow state), the DLQ (failed items needing inspection), and the export log (external audit stream).
- **Make the event store also the DLQ.** Mixing normal and dead-letter events in one store would complicate replay and make failure analysis harder.
