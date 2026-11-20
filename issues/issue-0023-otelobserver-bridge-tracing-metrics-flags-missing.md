+++
id = "I-023"
title = "OtelObserver bridge + --tracing/--metrics flags missing"
status = "done"
created = "2026-11-07"
priority = "med"
+++

# OtelObserver bridge + --tracing/--metrics flags missing

Status: **open** · Created: 2026-11-07

## Description

P6 addendum (ADR-0040). `peregrin-telemetry` has no `Observer` impl bridging engine events → OTEL; `--tracing`/`--metrics` provider flags absent from the CLI.

## Checklist

- [ ] See Description

## Resolution

**Done** (merged `7242ab6d`, 2026-11-20). New `peregrin-telemetry`
crate bridges the engine `Observer` port to OTLP per ADR-0040:
`OtelObserver` maps run/node lifecycle to spans (`campaign.run`
root, `node.<id>` children, `traceparent` propagation into
drivers), `OtelMetricsObserver` records runs/nodes/events counters
+ histograms, `CompositeObserver` in peregrin-bus fans out to both
signals. `--tracing`/`--metrics`/`--<signal>-provider` flags on
`campaign run`/`exec`/`submit`/`serve`; absent flags = noop.
Follow-up noted: only `otlp`/`noop` providers ship — `jsonl`/
`prometheus` destinations from ADR-0040 remain unimplemented.
