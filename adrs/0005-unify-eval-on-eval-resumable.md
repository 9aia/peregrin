# ADR 0005: Unify `eval` and `eval_async` on `eval_resumable`; Deprecate `AsyncFunction` as a Language Concept

## Status

Proposed

## Context

`ac-vm` currently exposes the sync/async color of Rust through the `VirtualMachine` and `Host` traits. `Host` has `function` and `async_function`; `CanonicalVm` has `eval` and `eval_async`. The two `eval` implementations are nearly identical, with `eval_async` awaiting on `AsyncFunction`. See `crates/vm-canonical/src/vm.rs` lines 58-300.

This design conflicts with ADR 0004: if the canonical language is colorless, the VM should not have a separate `AsyncFunction` path. A single `read()` call should be an `Operation` (effect), and the host driver should resolve it either synchronously or asynchronously. The VM should expose one evaluation primitive that supports both.

`eval_resumable` already captures all the needed machinery: it walks the expression tree, yields on unhandled `Operation`s, and returns a `Continuation`. Sync and async are just two ways to drive that loop.

## Decision

1. **Remove `AsyncFunction` from the canonical `Host` trait and the `ac-vm` public API.**
   - The canonical `Host` resolves names to `Function`, `LazyFunction`, `Operation`, and `GenericFunction` only.
   - `AsyncFunction` may remain in `ac-vm-common` as a host-side utility, but it is not a concept the language or core VM sees.

2. **Make `eval_resumable` the single core evaluation primitive.**
   - `VirtualMachine` exposes `eval_resumable`.
   - `eval` becomes a convenience wrapper: a driver that immediately resolves yielded operations synchronously (or blocks).
   - `eval_async` becomes a convenience wrapper: an async driver that awaits yielded operations and resumes.
   - Both call the same `eval_resumable` implementation.

3. **Implement the driver as an operation resolver.**
   - A sync driver: `match eval_resumable(...) { Done(v) => v, Yielded { value, continuation } => { let result = resolve_sync(value); loop again with result } }`.
   - An async driver: `match eval_resumable(...) { Done(v) => v, Yielded { value, continuation } => { let result = resolve_async(value).await; loop again } }`.

4. **Rename/refactor `CanonicalVm` so the `eval` and `eval_async` match-based code is replaced by the resumable loop.**
   - `EvalResult` is already the internal return type of `eval_resumable`.
   - The sync and async entry points are small drivers around the resumable engine.

5. **Update `ModuleHost` and `Module` trait accordingly.**
   - Remove `async_function` from `Module`.
   - Host modules expose `Operation` for side effects; the host driver is responsible for async execution.

## Consequences

### Positive

- Eliminates the duplicated `eval`/`eval_async` match blocks.
- Eliminates the `Function` vs `AsyncFunction` split in the host.
- The language becomes colorless at the VM layer, consistent with ADR 0004.
- A new host only needs to provide `Operation` for effects and a driver for sync or async.
- A `handle` can choose an interpretation by re-raising a different operation (e.g., `read_sync` vs `read_async`), but there is no `async` keyword in the language.
- Resumability, build/runtime staging, and streaming are all built on the same primitive.

### Trade-offs

- Existing hosts that use `AsyncFunction` must be refactored to provide `Operation`s and a driver.
- `eval` for a host with no operations becomes a simple pass-through, but for any operation it now runs the resumable loop.
- There is a small overhead: every function call now goes through the resumable frame stack machinery, even for pure sync calls. This can be optimized later, but the default path is now resumable.
- The `async` driver needs a way to poll `eval_resumable` while awaiting operations; this requires a loop that owns the `Continuation`.
- `LazyFunction` still exists and is not affected.

## Relationship to ADR 0004

This ADR implements the VM changes needed by ADR 0004. ADR 0004 states the design goal; this ADR changes the `ac-vm` API to make it real.

## References

- `adrs/0004-colorless-execution.md`
- `packages/ac-lang/crates/vm/src/host.rs`
- `packages/ac-lang/crates/vm/src/runtime.rs`
- `packages/ac-lang/crates/vm/src/operation.rs`
- `packages/ac-lang/crates/vm-canonical/src/vm.rs`
- `packages/ac-lang/crates/vm-canonical/src/continuation.rs`
- `packages/ac-lang/crates/runtime-core/src/module/module.rs`
- `packages/ac-lang/crates/runtime-core/src/host/host.rs`
