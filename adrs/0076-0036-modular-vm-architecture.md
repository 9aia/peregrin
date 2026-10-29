+++
id = "ADR-0076"
title = "0036 — Modular VM Architecture"
status = "open"
priority = "medium"
+++

# 0036 — Modular VM Architecture

## Status

Accepted

## Context

The `handl-vm` crate had grown into a monolithic block: the `Vm` implementation, the
`Host`/`Function`/`Operation` traits, the `Context`, the `Error` enum, and the
resumable-continuation machinery all lived in one package. This made the VM
hard to test in isolation, hard to replace for alternative evaluation strategies,
and hard to reason about: a single change in the continuation format required
touching the same crate that defines the public `Host` contract.

The lexer, parser, and AST layers already follow a cleaner shape:

- `handl-lexer` / `handl-lexer-common` / `handl-lexer-canonical`  
- `handl-parser` / `handl-parser-common` / `handl-parser-canonical`  
- `handl-ast` / `handl-ast-common` / `handl-ast-canonical`

Each has a thin `handl-XXX` facade that preserves the public API while the real
work is split between generic machinery (`-common`) and the canonical
implementation (`-canonical`). The VM should follow the same pattern.

## Decision

Split the VM into three crates and decouple the core abstractions from the
canonical tree-walking executor.

### Crate split

| Crate | Responsibility |
|-------|----------------|
| `handl-vm-common` | Generic VM traits and types: `Context`, `Error`, `Eval`, `Host`, `Function`, `Operation`, etc. No built-in functions, no parser, no I/O. |
| `handl-vm-canonical` | The canonical tree-walking VM, `Continuation`, `Frame`, and resumable evaluation. Builds on `handl-vm-common`. |
| `handl-vm` | A thin facade that re-exports the public surface from `handl-vm-common` and `handl-vm-canonical` so existing consumers do not need to change dependencies. |

### Interface decoupling

- The `Vm` contract becomes a `VirtualMachine` trait in `handl-vm-common`.
  `CanonicalVm` in `handl-vm-canonical` implements it.
- `Host` remains a trait, but the canonical implementation does not depend on
  concrete host types.
- `Context` is split: `handl-vm-common` exposes an abstract `RuntimeContext` trait,
  and `handl-vm-canonical` provides `CanonicalContext`. This removes the direct
  coupling between `Context` and `Box<dyn Eval>` while still supporting the
  callback pattern.
- `Error` becomes generic over the yielded value type, with a concrete
  `CanonicalError` alias tied to `NonFunctionValue` for the canonical crate.
- `Eval`/`AsyncEval` are isolated in `handl-vm-common` and used through the
  `RuntimeContext` trait, not via `Context` internals.

### Consequences

- **Composable**: A host can be assembled from small `Function`/`Operation`
  implementations without depending on the VM executor.
- **Modular**: `handl-vm-common` has no I/O, no parser, no AST serializer; it only
  defines the evaluation contract.
- **Replaceable**: A different evaluator (e.g., stack machine, JIT, async-only
  runtime) can implement `VirtualMachine` and `Host` without touching
  `handl-vm-canonical`.
- **Extensible**: New function kinds (lazy, generic, async) are added as traits
  in `handl-vm-common` and consumed generically by the canonical VM.
- **Testable**: Tests for the canonical VM live in `handl-vm-canonical`; mocks and
  fake hosts live in `handl-vm-common` test modules. The `Host` trait lets tests
  inject minimal hosts per scenario.
- **Performant**: The design relies on `&dyn` object-safe traits and monomorphized
  `CanonicalVm<H>` over `H: Host`, keeping the same zero-cost shape as the old
  monolith.

### Backwards compatibility

`handl-vm` re-exports the existing names (`Vm`, `Context`, `Error`, `Host`, …)
from the new crates. Where the same name now refers to a trait or a different
concrete type, the public API signature should remain source-compatible for
normal use. Internal module paths (`crate::context`, `crate::error`, etc.) are
no longer exposed, so callers that imported through those will need to use the
re-exports from `handl_vm`.

## References

- `handl-lexer` / `handl-lexer-common` / `handl-lexer-canonical`
- `handl-parser` / `handl-parser-common` / `handl-parser-canonical`
- `handl-ast` / `handl-ast-common` / `handl-ast-canonical`
