+++
id = "ADR-0078"
title = "0038 — Result Host Module"
status = "open"
priority = "medium"
+++

# 0038 — Result Host Module

## Status

Proposed

## Context

The handl runtime needs a first-class `result` type for expressions that can fail without using exceptions. This is inspired by:

- Rust's `Result<T, E>`
- TypeScript `better-result` and `neverthrow`
- Effect's error channel and `_tag`-based discriminated unions

Today the VM supports modules like `math`, `str`, and `fn` through the `Module`/`Function<NonFunctionValue>` pattern in `handl-runtime-modules`. A `result` module should follow the same shape so it can be composed into `ModuleHost` alongside the existing standard library.

## Decision

Add a `result` module to `handl-lib-utils` in the `handl` workspace.

- `ResultModule` implements `ac_runtime_modules::Module` under the namespace `result`.
- Each function is a unit struct implementing `Function<NonFunctionValue>`.
- The module re-uses `ac_runtime`, `ac_runtime_values`, `ac_runtime_modules`, and `serde_json`.

### Value representation

A `Result<T, E>` is encoded as a JSON object with a discriminated `_tag`:

- `{"_tag": "Ok", "value": <T>}`
- `{"_tag": "Err", "error": <E>}`

This matches the Effect-style discriminated unions already used elsewhere in the project and keeps `is_ok` / `is_err` cheap to implement.

### Exposed handl functions

Initial surface — the `result` module owns only constructors, predicates, and the eliminator:

- `result.ok(value)` — construct an `Ok`
- `result.err(error)` — construct an `Err`
- `result.is_ok(r)` — predicate
- `result.is_err(r)` — predicate
- `result.match(r, on_ok, on_err)` — the only eliminator

The general combinators are intentionally omitted because the existing `fn` module already provides the necessary composition tools:

- `map(r, f)` → `result.match(r, fn.flow(f, result.ok), result.err)`
- `map_err(r, f)` → `result.match(r, result.ok, fn.flow(f, result.err))`
- `and_then(r, f)` → `result.match(r, f, result.err)`
- `or_else(r, f)` → `result.match(r, result.ok, f)`
- `tap(r, f)` → `result.match(r, fn.tap(f, result.ok), result.err)`

Only `match` receives `FunctionValue`s and calls `ctx.eval_expr` with the bound value, mirroring the pattern in `lib-utils`.

### Implementation rules

- All function names exposed to handl are `snake_case`.
- Unused `Context` parameters use the `_ctx` naming convention from existing modules.
- No `unwrap`, `expect`, or `unwrap_or` helpers; `match` is the only eliminator, and collapse is composed with `fn`/`type` helpers.
- Invalid inputs return `Error::TypeMismatch` with explicit `expected`/`got` descriptions.
- No new crate or feature flag; the module lives inside `lib-utils`.

### Open / deferred

The constraint to "use `dude`" is recorded but not yet specified. This ADR will be updated once the `dude` integration point is defined.

## Consequences

### Positive

- handl expressions can represent and transform errors explicitly.
- Keeps the VM decoupled from modules; the VM only sees the `Host` contract.
- Reuses the established `Module`/`Function` patterns, so no new runtime machinery is needed.
- `_tag`-based representation is easy to inspect from other host languages and serializations.

### Trade-offs

- A `Result` is just a `serde_json::Value` object at runtime; there is no static type enforcement.
- `match` requires `FunctionValue` round-trips, which is slower than native branching, but it keeps the module small and lets `fn` own composition.
- The initial set is minimal; more exotic combinators (`partition`, `transpose`, etc.) can be added later.

### Backwards compatibility

This is a new module inside `lib-utils`, so no existing handl programs or workspace features change. Consumers opt in by adding `ResultModule` to their `ModuleHost`.

## References

- `crates/lib-utils/src/result/mod.rs`
- `crates/lib-utils/src/result/functions.rs`
- `crates/lib-utils/src/result/module.rs`
- `crates/lib-utils/src/fn/module.rs`
- `crates/runtime-core/src/lib.rs`
- `crates/host/src/lib.rs`
