# ADR 0002: Composable Lego-Style VM Architecture

## Status

Proposed

## Context

`handl-vm` defines the `VirtualMachine` trait and `handl-vm-canonical` provides a single, monolithic `CanonicalVm` that matches every `Expr` variant in one tree-walking `eval` method. While this works for the canonical language, it has several architectural limits:

1. **The VM is not composable from parts.** `CanonicalVm` owns the evaluation logic for `Value`, `Call`, `Array`, `Object`, and `Handle` all at once. A host that only wants a subset of the language (e.g. no `Handle`, no lazy calls, no async) must still carry and audit the entire implementation.
2. **Extension requires editing the monolith.** Adding a new expression variant, optimization, or dialect-specific behavior means opening `CanonicalVm::eval` and `CanonicalVm::eval_async` and threading changes through the entire tree walker.
3. **Middleware only observes, it does not recompose.** `MiddlewareVm` in `handl-vm-common` can wrap a VM with cross-cutting hooks (`Trace`, `Counter`, `Chain`), but it cannot change _which_ VM logic handles a given node. It is an outer wrapper, not an inner assembler.
4. **There is no path to alternative evaluators.** `VirtualMachine` is pluggable, but the workspace contains only `CanonicalVm` plus wrappers. A capability-specific VM (e.g. a pure-value machine, a strict no-`Handle` machine, a sandboxed machine) does not exist as a separate, reusable component.

The goal is to make the VM layer as composable as the host layer already is (`ChainHost`, `FilterHost`, `SpyHost`, `MapHost`) and the middleware layer (`MiddlewareVm`, `Chain`).

## Decision

Introduce an `ExprHandler` trait and a `LegoVm` dispatcher. `CanonicalVm` becomes a default _bundle_ of handlers rather than a single, closed implementation.

### `ExprHandler`

A handler is responsible for one expression shape or one language capability:

```rust
pub trait ExprHandler: Send + Sync + 'static {
    fn can_handle(&self, expr: &Expr<NonFunctionValue>) -> bool;

    fn eval(
        &self,
        expr: &Expr<NonFunctionValue>,
        ctx: &mut dyn Context,
        host: &dyn Host,
        dispatch: &mut dyn FnMut(&Expr<NonFunctionValue>, &mut dyn Context)
            -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error>;
}
```

`can_handle` lets the dispatcher select the right handler for a node. `dispatch` lets a handler recurse through the engine for sub-expressions without hardcoding the full evaluation loop. `Host` is passed explicitly because handlers are the layer that resolves function names, not the dispatcher.

### `LegoVm`

`LegoVm<H: Host>` owns an ordered list of handlers and a reference to the host. On `eval`, it scans the list and uses the first matching handler. The canonical implementation is registered last as the fallback:

```rust
pub struct LegoVm<H: Host> {
    host: H,
    handlers: Vec<Box<dyn ExprHandler>>,
}

impl<H: Host> LegoVm<H> {
    pub fn new(host: H) -> Self { ... }
    pub fn with(mut self, handler: impl ExprHandler) -> Self { ... }
}
```

`LegoVm` implements `VirtualMachine`.

### Canonical bundle

`handl-vm-canonical` exports a `CanonicalBundle` / `canonical_vm` constructor that registers the default handlers:

```rust
pub fn canonical_vm<H: Host>(host: H) -> LegoVm<H> {
    LegoVm::new(host)
        .with(ValueHandler)
        .with(CallHandler)
        .with(LazyCallHandler)
        .with(ArrayHandler)
        .with(ObjectHandler)
        .with(HandleHandler)
}
```

`CanonicalVm` can be preserved as a thin type alias or deprecated in favor of `canonical_vm`.

### How handlers compose

- **Priority composition** — `with` appends handlers; the first `can_handle` wins. This lets a dialect or security layer override canonical behavior without forking the whole VM.
- **Recursive dispatch** — handlers call `dispatch` to evaluate sub-expressions, so a `CallHandler` does not need to know how `Array` or `Handle` are implemented.
- **Capability-specific bundles** — hosts can assemble only the handlers they support, e.g. `pure_value_vm(host)` for data-only use or `sandboxed_vm(host)` that omits `Handle`.

### Relationship to `MiddlewareVm`

`MiddlewareVm` remains the cross-cutting concern layer. `LegoVm` is the structural composition layer. A production setup may use both:

```rust
let vm = MiddlewareVm::new(
    LegoVm::new(host)
        .with(ValueHandler)
        .with(SandboxedCallHandler)
        .with(ArrayHandler)
        .with(ObjectHandler),
    Chain::new().with(Box::new(Trace::new())),
);
```

## Consequences

### Positive

- The canonical VM is no longer a special, closed implementation; it is one bundle of handlers.
- New expression variants or language features can be added as a new `ExprHandler` without touching existing handlers.
- Hosts can restrict, extend, or replace individual evaluators (e.g. a custom `CallHandler` for sandboxing) without forking the whole tree walker.
- Handlers are independently unit-testable with mock `Host`s and stub `dispatch` closures.
- The dispatcher can add runtime instrumentation around each handler without coupling to `MiddlewareVm`.
- The `VirtualMachine` trait remains the boundary; `LegoVm` is just one implementation strategy.

### Trade-offs

- `Vec<Box<dyn ExprHandler>>` and `&mut dyn FnMut` dispatch per node adds virtual-call overhead compared to the closed `match` in `CanonicalVm`. This is acceptable for the targeted use cases (embeddable, host-defined languages) but not ideal for raw throughput. A hot-path VM can bypass `LegoVm` and implement `VirtualMachine` directly.
- The `Expr` enum remains the common data model. If a dialect needs radically different expression shapes, it must either extend `Expr` or implement a completely separate `VirtualMachine`. `ExprHandler` is not a new AST layer.
- `Host` is passed to every handler call. If a host is expensive to pass by reference, the dispatcher can hold `&Host` once and expose it. The current `&dyn Host` is the existing pattern.
- Converting `CanonicalVm::eval` and `CanonicalVm::eval_async` into separate handlers is a non-trivial refactor of `handl-vm-canonical`. It should be done in phases: introduce `LegoVm` and handlers in `handl-vm-common`, then migrate `CanonicalVm` to use them, then deprecate the old `match`-based code.

## References

- `packages/handl/crates/vm/src/runtime.rs` — `VirtualMachine` trait
- `packages/handl/crates/vm-canonical/src/vm.rs` — `CanonicalVm` tree-walker
- `packages/handl/crates/vm-common/src/middleware_vm.rs` — `MiddlewareVm` and `VmMiddleware`
- `packages/handl/crates/vm-common/src/host.rs` — host combinators (`ChainHost`, `SpyHost`, etc.)
