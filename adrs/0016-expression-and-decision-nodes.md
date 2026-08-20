# ADR-0016: Expression and decision nodes

## Status

Proposed

## Context

Many Peregrin examples implement branching and data transformation with `driver: shell`. Examples `02-branching.yaml`, `13-saga.yaml`, and `20-deploy-pipeline.yaml` use shell `if` statements, `exit 0`/`exit 1`, and `with.emit` maps to route events. This is flexible but verbose, not type-checkable, and mixes workflow logic with arbitrary process execution.

A declarative expression driver would let authors branch, transform, and set state without spawning a process.

## Decision

Introduce an `expr` driver that can evaluate conditions, transform payloads, set shared state, and emit different events based on the result.

### `choice` mode

```yaml
- id: route-alert
  driver: expr
  on: [alert.received]
  emits: [alert.critical, alert.minor]
  with:
    choice:
      - if: "event.payload.severity == 'critical'"
        emit: alert.critical
      - default:
        emit: alert.minor
```

### Transform mode

```yaml
- id: build-summary
  driver: expr
  on: [lint.passed, test.passed]
  mode: all
  emits: [summary.ready]
  with:
    set_state:
      status: ok
    emit:
      summary: "Lint: ${event.payload[0].stdout}; Tests: ${event.payload[1].stdout}"
```

### Filter mode

```yaml
- id: ignore-minor
  driver: expr
  on: [alert.received]
  emits: [alert.important]
  with:
    filter: "event.payload.severity in ['critical', 'high']"
    emit: alert.important
```

The expression language must support:
- References to `event`, `config`, `state`, `secret`, and `node` values.
- Common comparisons, logical operators, and list/string helpers.
- Template rendering in `emit` fields.

## Consequences

- Fewer shell nodes are needed for pure workflow logic.
- Branching and filtering become first-class and more amenable to static validation.
- The `ac typegen` and `ac validate` commands can check expression references at load time.
- A new package, `ac-driver-expr`, is needed.
- Care must be taken to keep the expression language small and predictable; it is not a general-purpose programming language.

## Alternatives considered

- **Use `driver: python` for all logic.** Rejected because it requires Python code for one-line decisions, undermining the declarative value of YAML workflows.
- **Keep shell as the only branching primitive.** Rejected because it is brittle, hard to validate, and overkill for simple conditions.
- **Adopt a full expression language such as jq or CEL.** Considered for a later phase; a small built-in grammar is preferable for the first version to avoid adding a heavy dependency.
