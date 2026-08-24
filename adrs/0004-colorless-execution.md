# ADR 0004: Colorless Execution — Async/Sync as a Host Concern

## Status

Proposed

## Context

The AC canonical language already has no `await`, no `async` keyword, and no built-in I/O. Its syntax is colorless. The runtime, however, still exposes the sync/async color of Rust: `Host` has `function` and `async_function`, `CanonicalVm` has `eval` and `eval_async`, and a host that wants both modes must duplicate logic. The runtime should match the syntax.

Arrow handlers (`Operation` + `handle`) already separate an operation request from its resolution. A `read()` call in the language is just an operation; the handler that catches it decides whether to block, await, or yield. This makes sync vs async a host interpretation, not a language feature. The same pattern generalizes to buildtime vs runtime and server vs browser: different hosts resolving the same operations in different environments.

Streaming and out-of-order rendering (e.g., prerender HTML on the server, resume client-only parts in the browser) require resumable boundaries, which `Continuation` already provides, and a host-provided streaming composition function.

## Decision

1. **Make the canonical AC language colorless with respect to sync and async.**
   - No `async`, `await`, or effect-coloring keywords in the language syntax.
   - All side-effecting or capability calls are `Operation`s.
   - Pure computation remains `Function`.

2. **Resolve async/sync at the host level.**
   - A single `Operation` named `read` (or `fs.read`, etc.) is the only canonical call.
   - The host driver decides whether to resolve the operation synchronously, asynchronously, or not at all (yield for a later host).
   - `handle` clauses in a program can select an operation interpretation by re-raising a different operation, but the outermost host is authoritative.

3. **Unify the VM on `eval_resumable`.**
   - `eval_resumable` is the canonical evaluation path.
   - A sync host is a driver that blocks on I/O and resumes.
   - An async host is a driver that awaits and resumes.
   - The `Continuation` is the contract between the stages; it is JSON-serializable and can move between hosts.

4. **Treat buildtime and runtime as different hosts.**
   - A build host partially evaluates a program, resolves operations it can (e.g., `import` of a local module, `read` of a known file, constant `if`), and yields the rest as a `Continuation`.
   - A runtime host resumes the `Continuation` with the missing operations.
   - The language and `Continuation` format are the same for both.

5. **Support streaming/out-of-order rendering through `handle` boundaries and a host-provided streaming composition function.**
   - A page is composed of `handle`-wrapped boundaries.
   - A streaming `concat`/`page` host function flushes completed boundaries and emits placeholders for yielded ones.
   - The client host later resumes each yielded `Continuation` and hydrates the placeholders.

6. **Deprecate `AsyncFunction` as a first-class language/VM concept.**
   - Async remains an implementation detail inside host drivers.
   - `Function` is for pure computation; `Operation` is for all effects.
   - Existing `AsyncFunction` may be preserved temporarily as a host convenience, but it is not exposed to the canonical language.

## Consequences

### Positive

- The canonical language does not need `async`/`await` syntax or a color system.
- The same program runs under sync, async, build, or runtime hosts without modification.
- Buildtime partial evaluation, server prerender, and client hydration are all the same mechanism: one host yields, another resumes.
- Streaming and out-of-order rendering become host concerns; the language only provides `handle` boundaries.
- Capability and sandboxing are centralized in the host, which is the authoritative outermost handler.

### Trade-offs

- The host becomes responsible for scheduling, async runtime, and streaming composition.
- `Operation` must be the canonical way to do I/O and effects; hosts must design their operation surface.
- `eval` and `eval_async` may remain as convenience entry points, but they become thin wrappers around `eval_resumable` with sync or async drivers.
- `AsyncFunction` removal/refactoring is a breaking change for existing host implementations.
- A `handle` clause cannot `await` directly; it must re-raise an operation to an async driver. This is a discipline, not a technical limitation.
- Static capability analysis can list possible operations, but dynamic calls and conditional branches still require runtime over-approximation.

## References

- `packages/ac-lang/docs/LANG.md`
- `packages/ac-lang/docs/explanations/design-principles.md`
- `packages/ac-lang/crates/vm/src/host.rs`
- `packages/ac-lang/crates/vm/src/operation.rs`
- `packages/ac-lang/crates/vm-canonical/src/continuation.rs`
- `packages/ac-lang/crates/vm-canonical/src/vm.rs`
- `packages/ac-lang/papers/declarative-topology-synthesis.md`
- `packages/ac-examples/examples/capability-confinement/README.md`
