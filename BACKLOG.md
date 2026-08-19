# Backlog

This file tracks known work that is not yet implemented. It is intentionally
low-ceremony; items are removed when they ship.

## DSL and architecture specification

A formal, language-agnostic description of the Agent Campaigns DSL, the engine
architecture, and the adapter ports. This is the contract that other language
runtimes must implement to be compatible with the same YAML workflows.

### Scope

- Full YAML schema with field semantics, defaults, and validation rules.
- Event graph model: node lifecycle, bus semantics, and ordering guarantees.
- Driver contract: signature, `ExecutionContext`, error handling, and allowed
  events.
- Adapter ports: `EventStore`, `Observer`, `DeadLetterQueue`, `ExportLog`,
  `SecretStore`, `Queue`.
- Resilience primitives: `retry`, `circuit_breaker`, `fan_in`, `poll`, `until`,
  `retire_on`.
- `strict` and bounded-defaults behavior.
- CLI surface and programmatic API.
- Test fixtures and golden workflows.

### Child ports

#### Python SDK port

Peregrin is being ported from Python to TypeScript. The runtime, drivers, CLI,
and core model are now TypeScript. A Python SDK does not currently exist.

##### Known impact

Many documentation files still show the old Python API:

- `docs/ac/how-to/programmatic-api.md` — imports `agent_campaign` Python packages.
- `docs/ac/how-to/write-a-custom-driver.md` — shows Python `Context`, `Event`, `Node`.
- `docs/ac/how-to/write-a-custom-observer.md` — shows Python observers.
- `docs/ac/how-to/write-a-custom-store.md` — shows Python stores.
- `docs/ac/how-to/custom-drivers.md` — Python handlers and `register()` API.
- `docs/ac/how-to/dynamically-inject-nodes.md` — Python `Context.inject_nodes()`.
- `docs/ac/explanation/handlers.md` — describes Python handlers as the primary handler model.
- `docs/ac/explanation/adapter-ports.md` — Python port examples.
- `docs/ac/reference/handler-registration.md` — Python `register()` examples.
- `docs/ac/reference/otel-observer.md` — Python `OTelObserver`.
- `docs/ac/reference/memory-store.md` — Python `MemoryStore`.
- `docs/ac/reference/json-store.md` — Python `JsonStore`.
- `docs/ac/reference/dead-letter-queue.md` — Python DLQ example.
- `docs/ac/reference/handler-context.md` — Python handler context.
- `docs/ac/reference/performance.md` — Python benchmark numbers.

These pages either need to be rewritten for TypeScript or moved behind a
"Python SDK" section once the Python SDK is ported.

##### What is still Python

The `python` driver (`packages/ac-driver-python`) still exists in the
TypeScript runtime. It spawns the `python` interpreter with `with.script`,
`with.module`, and `with.function`. This is a shell-out, not a Python SDK.
The reference docs for `driver: python` should reflect that.

##### Proposed Python SDK scope (future)

- A Python package (e.g. `peregrin` or `agent-campaigns`) that mirrors the
  TypeScript `ac-core` API.
- `Workflow`, `Node`, `Event`, `Runner` classes.
- Programmatic workflow execution.
- Python handlers that are first-class drivers.
- Python observer and store implementations.

#### Go SDK port (future)

A future port of the Agent Campaigns runtime to Go. It should consume the same
YAML workflows and implement the same adapter ports, taking advantage of Go's
concurrency model for the event bus and driver isolation.

### Generators

A `peregrin generate` command and `packages/ac-generator` tooling that turns a
declarative YAML workflow into deterministic artefacts: Mermaid/Graphviz
diagrams, Markdown runbooks, event schema stubs, test skeletons, prompt
templates, and aggregated campaign maps for workflow libraries. See
[ADR-0006: Campaign generators](/adrs/0006-generators.md).

## Doc audit: Python references

- The `python` driver reference (`docs/ac/reference/drivers/python.md`) needs
  to describe the TypeScript driver's `with.script` / `with.module` /
  `with.function` interface.
- `docs/ac/explanation/handlers.md` needs to be rewritten around TypeScript
  drivers, or renamed to "Drivers".
- `docs/ac/reference/handler-registration.md` should become a TypeScript
  "Registering a custom driver" guide.
- `docs/ac/how-to/programmatic-api.md` should use the TypeScript package API
  (not yet public).
- `docs/ac/reference/performance.md` should be regenerated from the TypeScript
  runtime or removed until benchmarks exist.
