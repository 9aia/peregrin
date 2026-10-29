# Backlog

## DSL and architecture specification

## Scope

- T-006 Full YAML schema params field semantics, defaults, and validation rules.
- T-007 Event graph model: node lifecycle, bus semantics, and ordering guarantees.
- T-008 Driver contract: signature, ExecutionContext , error handling, and allowed events.
- T-009 Adapter ports: EventStore , Observer , DeadLetterQueue , ExportLog , SecretStore , Queue .
- T-010 Resilience primitives: retry , circuit\_breaker , fan\_in , poll , until , retire\_on .
- T-011 strict and bounded-defaults behavior.
- T-012 CLI surface and programmatic API.
- T-013 Test fixtures and golden workflows.

## Child ports

## Python SDK port

## Known impact

## Many documentation files still show the old Python API:

- T-014 docs/handl/how-to/programmatic-api.md — imports agent\_campaign Python packages.
- T-015 docs/handl/how-to/write-a-custom-driver.md — shows Python Context , Event , Node .
- T-016 docs/handl/how-to/write-a-custom-observer.md — shows Python observers.
- T-017 docs/handl/how-to/write-a-custom-store.md — shows Python stores.
- T-018 docs/handl/how-to/custom-drivers.md — Python handlers and register() API.
- T-019 docs/handl/how-to/dynamically-inject-nodes.md — Python Context.inject\_nodes() .
- T-020 docs/handl/explanation/handlers.md — describes Python handlers as the primary handler model.
- T-021 docs/handl/explanation/adapter-ports.md — Python port examples.
- T-022 docs/handl/reference/handler-registration.md — Python register() examples.
- T-023 docs/handl/reference/otel-observer.md — Python OTelObserver .
- T-024 docs/handl/reference/memory-store.md — Python MemoryStore .
- T-025 docs/handl/reference/json-store.md — Python JsonStore .
- T-026 docs/handl/reference/dead-letter-queue.md — Python DLQ example.
- T-027 docs/handl/reference/handler-context.md — Python handler context.
- T-028 docs/handl/reference/performance.md — Python benchmark numbers.

## What is still Python

## Proposed Python SDK scope (future)

- T-029 A Python package (e.g. peregrin or agent-campaigns ) that mirrors the TypeScript handl-core API.
- T-030 Workflow , Node , Event , Runner classes.
- T-031 Programmatic workflow execution.
- T-032 Python handlers that are first-class drivers.
- T-033 Python observer and store implementations.

## Go SDK port (future)

## Exporters

## Scaffolders

## Install and registries

## Annotations

## Versioning

## Web UI

## Resolver: additional ModuleResolver implementations

## handl-resolver-git

## handl-resolver-http

## Doc audit: Python references

- T-034 The python driver reference ( docs/handl/reference/drivers/python.md ) needs to describe the TypeScript driver's params.script / params.module / params.function interface.
- T-035 docs/handl/explanation/handlers.md needs to be rewritten around TypeScript drivers, or renamed to "Drivers".
- T-036 docs/handl/reference/handler-registration.md should become a TypeScript "Registering a custom driver" guide.
- T-037 docs/handl/how-to/programmatic-api.md should use the TypeScript package API (not yet public).
- T-038 docs/handl/reference/performance.md should be regenerated from the TypeScript runtime or removed until benchmarks exist.
