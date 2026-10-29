+++
id = "ADR-0032"
title = "ADR-0032: User-defined workflow fields via `*.field.yml`"
status = "open"
priority = "medium"
+++

# ADR-0032: User-defined workflow fields via `*.field.yml`

## Status

Superseded by [handl ADR-0016](../../packages/handl/project/adrs/0016-custom-fields.md). Custom fields are now a language extensibility feature.

Proposed

## Context

The Peregrin workflow language is accumulating fields that are not true control-flow primitives: `timeout`, `delay`, `debounce`, `max_retries`, `cache`, `notify`, etc. Each new field currently requires a change to the engine and a new built-in. This couples high-level conveniences to the core, slows iteration, and makes the engine larger than it needs to be.

We already use YAML to extend the system in other dimensions: `.node.yml` files for reusable nodes (ADR-0029), `.snippet.yml` for reusable values, `.fn.yml` for custom functions (ADR-0024), and `.driver.yml` for driver presets (ADR-0026). A similar pattern for fields would let the community add new keywords without touching the engine.

## Decision

Support user-defined workflow fields declared in `*.field.yml` files. The core engine keeps a minimal set of built-in, low-level fields and operations; all higher-level or convenience fields are loaded from these declarative files.

### Core built-in fields

The core only implements the primitives needed to execute any workflow. Examples:

- `on:` — event/trigger binding
- `run:` — shell command execution
- `if:` — conditional branch
- `seq:` / `par:` — sequential and parallel composition
- `race:` — first-to-complete-cancels-others
- `sleep:` — wait for a duration
- `fail:` — abort with a message
- `params:` / `emit:` — context and event emission

All other fields are user-defined or library-defined.

### Field definition files

A `*.field.yml` file declares a field and tells the engine how to handle it.

```yaml
# fields/delay.field.yml
name: delay
type: sugar
schema:
  type: object
  required: [duration]
  properties:
    duration: { type: string }
expand: |
  seq:
    - sleep: duration
    - {{ step }}
```

```yaml
# fields/timeout.field.yml
name: timeout
type: sugar
schema:
  type: object
  required: [after]
  properties:
    after: { type: string }
    on_timeout: { type: string }
expand: |
  race:
    - {{ step }}
    - seq:
        - sleep: after
        - fail: on_timeout ?? "timed out"
```

```yaml
# fields/debounce.field.yml
name: debounce
type: handler
schema:
  type: object
  required: [wait]
  properties:
    wait: { type: string }
    run: { type: object }
    trailing: { type: boolean }
handler: "./handlers/debounce"
```

### Field types

- `sugar` — the field expands to a core-language tree before execution. No runtime code is required. Best for `delay`, `timeout`, `max_retries`, `cache`, `notify`, and other combinators that can be expressed with the core primitives.
- `handler` — the field is implemented by a TypeScript handler registered at load time. The YAML file is the manifest; the handler is the implementation. Best for stateful or time-sensitive fields like `debounce`, `rate_limit`, or `throttle`.

### Resolution

The engine resolves a field in this order:

1. Built-in fields.
2. `*.field.yml` files discovered in the workflow directory, then `libraries/fields/`, then configured field paths.
3. If no field is found, the workflow fails validation with a list of known fields.

### Location and naming

- Field files can live in a workflow directory, in `libraries/fields/<domain>/`, or in a project's `fields/` directory.
- The file extension is `.field.yml` or `.field.yaml`.
- Field names are global within a workflow; use dotted names for library fields to avoid collisions, e.g. `time.delay` or `resilience.timeout`.

### Promotion path

A field that starts as a `sugar` or `handler` can be promoted to a built-in field in the TypeScript engine when it is:

- Used in a performance-critical path.
- Adopted widely across projects.
- Proven safe and stable over time.

This mirrors the promotion path for custom functions (ADR-0024): built-ins emerge from the field library.

## Consequences

- The engine core stays small. New conveniences do not require engine changes.
- Users can create domain-specific fields without publishing a package or merging code.
- The same mechanism supports first-party library fields, so `timeout`, `delay`, `debounce`, and `max_retries` can ship as `libraries/fields/` files.
- Tooling must be built or extended to discover, validate, and render `*.field.yml` files in LSP, hover, and `handl info` output.
- Stateful fields require a handler interface, which adds a small but real runtime API surface.
- Error messages for sugar fields must be attributed back to the original field name, not the expanded core tree, to keep debugging comprehensible.
- Field name collisions become possible; dotted namespacing and load-order rules are required.

## Alternatives considered

- **Keep all fields as built-ins.** Rejected. It bloats the engine and slows the addition of new conveniences.
- **Allow arbitrary JavaScript in field definitions.** Rejected. It reintroduces the security and sandboxing risks that the whitelisted built-in model is designed to avoid.
- **Implement every field as a `node.yml` partial.** Rejected. Fields are syntactic sugar on a single step or expression, not reusable sub-workflows. Forcing them into nodes adds boilerplate and obscures the syntax.
