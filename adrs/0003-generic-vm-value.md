# ADR 0003: Generic, Pluggable Value and Function Representation for handl-vm

## Status

Accepted

## Context

The handl architecture states that the language kernel is value-agnostic: the
VM should not know what a String, Number, or User is, and the value
representation should be replaceable. The README describes a clean boundary
where `handl-ast` is generic, `Schema<V>` is abstract, and the VM "evaluates
expressions that produce `NonFunctionValue`s".

In reality the boundary is not clean. `NonFunctionValue` is hard-coded into the
VM and related traits:

- `handl-vm` defines `Function`, `AsyncFunction`, `LazyFunction`, `Operation`,
  `Context`, and `VirtualMachine` in terms of `NonFunctionValue`.
- `Error` in `handl-vm` carries `NonFunctionValue` for `Yield` and `Operation`.
- `handl-vm` depends directly on `handl-runtime-values` and `serde_json`.

`FunctionValue` is also a concrete, non-replaceable encoding for first-class
functions:

- It is a fixed JSON object with `params`, optional `return`, and `body`
  fields.
- `NonFunctionValue` detects functions by the presence of a `body` string.
- `handl-runtime-type` is building the type system directly on this single
  `FunctionValue` shape.

Because `NonFunctionValue` is the only value the VM understands, swapping it
for a compact binary value, a content-addressed value, a typed value, or a GPU
buffer would require editing the VM. Because `FunctionValue` is the only
first-class function encoding, adding closures, bytecode, or typed functions
would require changes to `handl-runtime-values` and to every consumer that
manually decodes `FunctionValue` JSON.

`handl-ast` (`Expr<V>`) and `handl-schema` (`Schema<V>`) are already generic and show
that the pattern works. We should extend the same pattern to the VM and to
first-class functions.

## Decision

1. Make `handl-vm` generic over a value type `V`.

   All VM traits and types become `V`-parameterized. `V` is constrained only
   by the small set of properties the VM needs to move values around:

   ```rust
   pub trait Value: Clone + Debug + Send + Sync + 'static {}
   ```

   Host functions and the `VirtualMachine` work in terms of `&[V]` and `V`:

   ```rust
   pub trait Function<V> {
       fn call(
           &self,
           args: &[V],
           ctx: &mut dyn Context<V>,
       ) -> Result<V, Error<V>>;
   }

   pub trait VirtualMachine<V> {
       fn eval(
           &self,
           expr: &Expr<V>,
           ctx: &mut dyn Context<V>,
       ) -> Result<V, Error<V>>;
   }
   ```

   `Context<V>`, `Host<V>`, `AsyncFunction<V>`, `LazyFunction<V>`, and
   `Operation<V>` are analogously generic. `Error<V>` carries `V` for `Yield`
   and `Operation` payloads.

2. Introduce a `FunctionRepr<V>` trait in `handl-vm-common`.

   The _core_ `handl-vm` does not need this trait, but the toolbox does:
   host functions like `apply`, `zip_all`, and `is_compatible` need to call a
   function value. `FunctionRepr` is the minimal interface they need:

   ```rust
   pub trait FunctionRepr<V>: Debug + Send + Sync + 'static {
       fn params(&self) -> &[(String, V)];
       fn return_schema(&self) -> Option<&V>;
       fn body(&self) -> &str;
   }
   ```

   This trait lives in `handl-vm-common`, not in `handl-vm` and not in any concrete
   value crate. Not every VM needs the concept of a first-class function, so
   it should not be in the core. A value type can expose a `FunctionRepr`
   through an `AsFunction` or `FunctionCodec` trait, also supplied by
   `handl-vm-common`.

3. Move `NonFunctionValue` and `FunctionValue` to a new `handl-vm-config` crate.

   The JSON-backed `NonFunctionValue` and `FunctionValue` are not the
   universal default for all handl uses. They are the concrete value model
   for the _configuration-language_ use case (YAML/JSON/TOML documents,
   `body:`-string functions). They belong in a dedicated `handl-vm-config` crate.

   `FunctionValue` implements `FunctionRepr<NonFunctionValue>` there.
   `handl-vm` and `handl-vm-common` do not depend on `handl-vm-config`. The CLI,
   frontends, and config-language tooling depend on it.

4. Remove function-value detection methods from `NonFunctionValue`.

   Today `NonFunctionValue` has `is_function()`, `as_function()`, and
   `new_function()` methods that hard-code the `FunctionValue` JSON shape.
   This makes the generic value type aware of one specific function
   representation. Instead, the conversion from a value to a `FunctionRepr`
   should be explicit and pluggable, for example:

   - `FunctionValue::try_from(value)` implemented in `handl-vm-config`.
   - An `AsFunction` or `FunctionCodec` trait in `handl-vm-common`.
   - A decoder that the host or a library layer selects.

   `NonFunctionValue` should not know what a function is; that interpretation
   belongs to `handl-vm-config` and its chosen `FunctionRepr` decoder.

## Consequences

### Positive

- `handl-vm` can run with any value type that implements `Value` (or the chosen
  bounds): compact binary, content-addressed, typed, or otherwise.
- `FunctionValue` is one replaceable implementation of `FunctionRepr`; other
  encodings (bytecode, closures, typed functions) can be added without
  touching the VM.
- The architecture goals from the README — runtime-agnostic, schema-agnostic,
  and format-agnostic — are realized for the value layer as well.
- `runtime-type` can move from the concrete `FunctionValue` JSON shape to a
  typed function representation behind the same `FunctionRepr` interface.
- The configuration-language default (`NonFunctionValue` + `FunctionValue`)
  continues to exist in `handl-vm-config`, so existing CLI and config tooling is
  preserved.

### Trade-offs

- Almost every `handl-vm` type gains a `V` parameter. We should introduce a
  `Value` trait or alias so the bounds are not repeated in every signature.
- `Error` must become generic over `V`, which propagates `V` into every
  `Result`. A default `Error<NonFunctionValue>` type alias can keep the
  default case ergonomic.
- Canonical host functions (`add`, `mul`, `len`, `concat`, etc.) currently
  use `NonFunctionValue` methods (`as_number`, `as_bool`, `as_str`). If a
  different `V` is chosen, those host functions will not automatically work.
  This is acceptable: host functions define value semantics for a specific
  value type. A different `V` needs its own host functions.
- `MiddlewareVm` and middleware combinators need to be generic over `V` as
  well.
- Existing tests and examples must be updated to pass `NonFunctionValue` as
  the explicit `V` and to import it from `handl-vm-config` instead of
  `handl-runtime-values`.

### Relationship to ADR 0002

ADR 0002 turns the canonical VM into a bundle of `ExprHandler`s. Combining
0002 with this ADR means handlers are `ExprHandler<V>`: the same structural
composition, now also generic over the value type.

## References

- `packages/handl/README.md` — stated value-agnostic and runtime-agnostic goals
- `packages/handl/crates/runtime/src/value.rs` — current `NonFunctionValue` and `FunctionValue`
- `packages/handl/crates/vm/src/host.rs` — `Function` and `Host` tied to `NonFunctionValue`
- `packages/handl/crates/vm/src/runtime.rs` — `Context` and `VirtualMachine` tied to `NonFunctionValue`
- `packages/handl/crates/ast-canonical/src/expr.rs` — already-generic `Expr<V>`
- `packages/handl/crates/runtime-type/src/lib.rs` — `FunctionValue`-based type checking proof-of-concept
- `adrs/0002-lego-vm-composition.md` — composable `ExprHandler` VM design
- (new) `handl-vm-common` — toolbox crate for `FunctionRepr` and helper traits
- (new) `handl-vm-config` — configuration-language crate for `NonFunctionValue` and `FunctionValue`
