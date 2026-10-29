+++
id = "ADR-0037"
title = "ADR-0037: VM Build-Kit and Categorical Canonical Semantics"
status = "open"
priority = "medium"
+++

# ADR-0037: VM Build-Kit and Categorical Canonical Semantics

## Status

Accepted

## Context

The modular VM split (`handl-vm` core traits, `handl-vm-common` reusable blocks, `handl-vm-canonical` reference interpreter) gave us a clean separation of concerns. However, `handl-vm-common` was initially only a thin helper crate with mocks and a few combinators. Production consumers need a much richer, off-the-shelf build-kit for composing custom virtual machines, host environments, and execution policies without re-implementing the same wiring every time.

At the same time, `handl-vm-canonical` should not be an arbitrary implementation. It should be the *categorical* reference semantics for the VM language: a pure, tree-walking interpreter that faithfully realizes the monad-like structure of effects (errors, yields, operations, handlers, async) on top of the untyped value world. Other interpreters can be swapped in, but `handl-vm-canonical` is the "category of the VM" in the same way `handl-ast-canonical` is the canonical abstract syntax.

## Decision

1. `handl-vm` remains the minimal core.
   - It defines the categorical objects and arrows: `Context`, `VirtualMachine`, `Function`, `Operation`, `Host`, `Eval`, etc.
   - It intentionally says *nothing* about how to compose, decorate, or optimize them.

2. `handl-vm-common` becomes the **VM build-kit**.
   - It is a Lego library of professional, production-ready, composable building blocks.
   - Blocks are grouped by the thing they compose:
     - `machine` — `Machine<V: VirtualMachine>` and `ContextBuilder`.
     - `builder` — closure-backed `Function`, `Operation`, `AsyncFunction`, `GenericFunction`, `LazyFunction` factories.
     - `combinator` — `Host` composition: `ChainHost`, `FilterHost`, `SpyHost`.
     - `middleware` — `Host` decorators: `MapHost`, `LoggingHost`, `AllowHost`.
     - `adapter` — `Function`/`Operation` combinators: `Memoize`, `MapArgs`, `MapResult`, `Catch`, `Pipe`, `Logged`, `Timed`, `CatchOp`.
     - `mock` — test doubles: `MockHost`, `NoopHost`, `CaptureFn`.
     - `sandbox` — resource-limit helpers.
   - New blocks may be added as new composition needs appear; the crate stays generic (no built-in language semantics).

3. `handl-vm-canonical` is the **categorical reference interpreter**.
   - It is the canonical implementation of `VirtualMachine`.
   - It realizes the categorical structure of the VM language in a pure, deterministic, tree-walking interpreter.
   - Other VMs must satisfy the same `handl-vm` traits; they are free to be compiled, JIT, stack-based, or partial, but `handl-vm-canonical` is the ground truth for semantics.

4. Composition is preferred over inheritance.
   - `Machine`, `Host`, and `Function` are all composed from smaller, independent blocks.
   - `Context` is configured via `ContextBuilder` so that every evaluation starts from a fresh, explicitly constructed environment.
   - `Host` behavior is assembled with `ChainHost`, `MapHost`, `LoggingHost`, `AllowHost`, etc.
   - `Function` behavior is assembled with `Memoize`, `MapArgs`, `Catch`, `Pipe`, etc.

## Consequences

- Consumers can build domain-specific VMs without writing an evaluator. They pick a `VirtualMachine` (usually `CanonicalVm`), a `Host`, a `ContextBuilder`, and a set of adapters.
- Tests can mock any layer independently: context, host, function, operation, or the whole VM.
- The canonical interpreter stays focused on correct semantics. Build-kit complexity lives in `handl-vm-common`.
- New composition patterns (e.g., new `Host` middleware, `Function` transformers, `Machine` variants) can be added to `handl-vm-common` without touching `handl-vm-canonical` or `handl-vm`.
- We accept a small amount of duplication in the build-kit (e.g., `Logged` for functions, `LoggingHost` for hosts) because each block is tuned for its layer.

## Examples

A consumer VM:

```rust
use handl_vm_common::prelude::*;
use handl_vm_canonical::CanonicalVm;

let host = MockHost::new()
    .with_function("add", fn_function(|args, _ctx| {
        let s: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(NonFunctionValue::new(s))
    }));

let machine = Machine::new(CanonicalVm::new(host))
    .with_context(|b| b
        .with_max_depth(64)
        .with_max_calls(1024)
        .with_namespace("x", NonFunctionValue::new(42)));

let result = machine.eval(&expr)?;
```

A wrapped function:

```rust
let f = pipe(
    fn_function(|args, _| Ok(args[0].clone())),
    map_result(|r| r.map(|v| NonFunctionValue::new(v.as_number().unwrap_or(0.0) * 2.0))),
);
```
