+++
id = "ADR-0055"
title = "ADR-0017: Declarative compensation"
status = "open"
priority = "medium"
+++

# ADR-0017: Declarative compensation

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

The `13-saga.yaml` example demonstrates a multi-step transaction with compensation. Each step has its own `*.failed` event, and separate `cancel-*` nodes listen to those events and to earlier compensations in reverse order. The graph is correct but fragile: the order of compensations is encoded implicitly in the topology, and every new step requires new compensation nodes and emit wiring.

A saga is a common enough pattern that it should be declarative. The workflow author should declare the forward action and its undo on the same node, and the engine should run the undo actions in the correct reverse order when the workflow fails.

## Decision

Add a `compensate:` field to nodes. The `compensate` block declares an undo action that runs when a `workflow.failed` or `compensate` event occurs, in the reverse order of the original successful firings.

```yaml
name: book-order

nodes:
  - id: reserve
    driver: shell
    on: [start]
    emits: [reserve.done, reserve.failed]
    compensate:
      on: [workflow.failed]
      driver: shell
      params:
        cmd: "release-reservation ${event.payload.reservation_id}"
    params:
      cmd: "reserve-inventory ${config.sku}"

  - id: charge
    driver: shell
    on: [reserve.done]
    emits: [charge.done, charge.failed]
    compensate:
      on: [workflow.failed]
      driver: http
      params:
        method: POST
        url: "https://payments.example.com/refund"
        json:
          charge_id: "${event.payload.charge_id}"
    params:
      cmd: "charge-card ${event.payload.reservation_id}"

  - id: ship
    driver: shell
    on: [charge.done]
    emits: [order.shipped]
    params:
      cmd: "ship-order ${event.payload.reservation_id}"
```

If `charge` fails, the engine runs `ship`'s compensation (nothing), then `charge`'s refund, then `reserve`'s release, in reverse firing order. It emits `compensation.complete` when all declared compensations have run, or `compensation.failed` if a compensation itself fails.

## Consequences

- Sagas are written as linear forward steps with inline undo, not as separate reverse graphs.
- The engine, not the author, guarantees reverse order and failure handling.
- Strict mode can verify that every step with side effects declares a `compensate` block.
- Existing manual saga graphs continue to work; `compensate` is optional.

## Alternatives considered

- **Keep manual `cancel-*` nodes.** Rejected because it is error-prone and does not scale with the number of steps.
- **Implement compensation as a separate `compensation` driver that runs after `workflow.failed`.** Rejected because tying the undo to the original node is clearer and makes strict-mode validation possible.
- **Use `runner` to inject compensations at runtime.** Rejected because it adds unnecessary indirection for a pattern that can be inferred from the static graph.
