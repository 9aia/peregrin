+++
id = "ADR-0049"
title = "ADR-0011: Web UI for Peregrin"
status = "open"
priority = "medium"
+++

# ADR-0011: Web UI for Peregrin

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

Peregrin is primarily a CLI and library tool. It runs workflows from YAML,
persists campaigns in an event store, and exposes engine state through the
`Observer` and `EventStore` ports. While the CLI and YAML-first approach work
well for local development and CI, a web UI is useful for:

- **Visibility.** Teams want to see running campaigns, node status, and the
  event log without reading JSON or YAML.
- **Human-in-the-loop.** The `human` driver currently prompts on `stdin`. A web
  UI can turn approval and input requests into a browser-based form.
- **Operations.** SREs and platform teams need a dashboard for failure traces,
  metrics, and the dead-letter queue.
- **Debugging.** A visual event graph and a step-by-step replay make it easier
  to understand why a campaign ended in a particular state.

A web UI should not be required to run a campaign. It is an optional layer on
top of the engine.

## Decision

Peregrin will provide an optional web UI package (`apps/webui` or
`packages/handl-webui`) that connects to a running engine or reads a campaign store
after the fact. The UI is a separate process or app; it does not execute the
engine itself.

### Architecture (proposed)

```text
┌─────────────┐     Observers / EventStore     ┌─────────────┐
│  Web UI     │  ──────  WebSocket / API  ──── │  Peregrin   │
│  (React)    │                                  │  engine     │
└─────────────┘                                  └─────────────┘
        │
        ▼
  handl-store-*
  (read-only for replay)
```

- The UI connects to the `Observer` port for live events and node spans.
- It reads `EventStore` and `DeadLetterQueue` for campaign replay and
  debugging.
- It writes events back to the engine only for human-in-the-loop actions such
  as approvals or form submissions.
- It can run in two modes:
  1. **Live** — attached to a running `Runner` and updated via `Observer`.
  2. **Read-only** — pointed at a store path to inspect a completed campaign.

### Features (proposed)

1. **Campaign list.** Show active and completed campaigns, their status, and
   the workflows they are running.
2. **Event graph.** Render the workflow as a live, interactive Mermaid or
   canvas graph using the exporter (ADR-0006).
3. **Event log.** A searchable, filterable view of the append-only event log.
4. **Node inspector.** Show inputs, outputs, triggers, retries, and errors for
   a node firing.
5. **Human approvals.** Turn `human` driver prompts into web forms and route
   responses back as events.
6. **Replay / recovery.** Reconstruct campaign state from the event log and
   step through transitions.
7. **Metrics.** Display Prometheus-style metrics captured by observers.

### Backend transport (proposed)

- A small `packages/handl-webui-server` package exposes a read/write API over
  WebSocket and HTTP.
- It reuses the existing `Observer`, `EventStore`, `DeadLetterQueue`, and
  `Runner` ports.
- Writes (approvals, manual events) go through the same `Runner` or `Bus`
  layer as CLI and driver events.

### Command surface (proposed)

```bash
handl webui --store .peregrin/store.json    # read-only mode
handl webui --attach --workflow deploy       # live mode
```

In embedded mode, `apps/webui` can be started as a standalone Next.js or Vite
application.

## Consequences

- The engine ports (`Observer`, `EventStore`, `DeadLetterQueue`) become
  first-class API contracts. This is consistent with ADR-0002.
- The `human` driver should expose a `request` event that the Web UI can render
  as a form. The same event can be answered from the CLI.
- The Web UI can reuse the exporters from ADR-0006 for diagrams and runbooks.
- It does not replace the CLI or YAML-first workflow. It is an operational
  companion.
- The Web UI can use `annotations` from ADR-0009 (superseded by handl ADR-0011) to display workflow and node
  titles, descriptions, examples, and documentation links.
- The Web UI can use exporters from ADR-0006 to render the event graph and
  runbook views.
- Security matters: the Web UI must respect the same capability and secret
  checks as the CLI. It is not a way to bypass the engine.

## Alternatives considered

- **Use a generic OpenTelemetry / Grafana dashboard.** Rejected because
  workflow-specific concepts (nodes, events, approvals) are not visible in
  generic traces.
- **Build the UI inside the CLI process.** Rejected because it would couple the
  engine to a web framework. A separate app keeps the engine small and allows
  remote operation.
- **Only provide a read-only web UI.** Rejected because human-in-the-loop
  approvals are one of the strongest reasons to have a UI.
