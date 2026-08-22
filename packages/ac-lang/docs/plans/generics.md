# Plan: Parametric Polymorphism (Generics) for AC-Lang Type System

**Status:** Implemented
**Date:** 2026-08-22

## Problem

The type system is monomorphic. Functions like `map`, `filter`, `reduce`, `apply`,
and user-defined generic functions cannot express parametric signatures. The
`Unknown` escape hatch absorbs all polymorphic call sites, losing static type
safety on the result.

## Scope

Generics are a **typechecker-only** feature. No runtime changes — the VM, AST,
host traits, value types, and continuation model are untouched. This is
consistent with the small-kernel principle: the kernel stays small, the static
analysis layer gets richer.

## Design decisions

### D1: Type variables in the `Type` enum

Add `Type::Var(String)` to represent named type variables (`T`, `U`, etc.).

```rust
pub enum Type {
    String,
    Number,
    Boolean,
    Null,
    Array(Box<Type>),
    Object(BTreeMap<String, Type>),
    Function(FunctionType),
    Var(String),       // NEW — type variable (e.g. "T")
    Unknown,
    Never,
}
```

`Var` is **not** compatible with concrete types via `is_compatible`. It only
unifies during generic call instantiation.

### D2: Type parameters on `FunctionType`

```rust
pub struct FunctionType {
    pub type_params: Vec<String>,      // NEW — e.g. ["T", "U"]
    pub params: Vec<(String, Type)>,
    pub return_type: Box<Type>,
}
```

A function with empty `type_params` is monomorphic — the common case. No
behavioral change for existing code.

### D3: Schema syntax convention

JSON Schema has no generics. We introduce a convention:

```json
{
  "type": "function",
  "typeParams": ["T", "U"],
  "params": [
    { "name": "arr", "schema": { "type": "array", "items": { "type": "var", "name": "T" } } },
    {
      "name": "fn",
      "schema": {
        "type": "function",
        "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
        "return": { "type": "var", "name": "U" }
      }
    }
  ],
  "return": { "type": "array", "items": { "type": "var", "name": "U" } }
}
```

In YAML (more readable for AC documents):

```yaml
type: function
typeParams: [T, U]
params:
  - name: arr
    schema: { type: array, items: { type: var, name: T } }
  - name: fn
    schema:
      type: function
      params:
        - name: x
          schema: { type: var, name: T }
      return: { type: var, name: U }
return: { type: array, items: { type: var, name: U } }
```

The `type: var` schema keyword is an AC extension to JSON Schema, interpreted
by `JsonSchemaInterpreter`. Other schema backends can use their own convention.

### D4: Unification-based call checking

Replace the structural `is_compatible` check in `check_call` with a
unification step when `type_params` is non-empty:

1. Create a fresh substitution map: `HashMap<String, Type>`.
2. For each `(param_type, arg_type)` pair, unify `arg_type` against `param_type`
   under the current substitution. If a `Var` is encountered, bind it (or check
   consistency if already bound).
3. Apply the final substitution to `return_type` and return the concrete result
   type.

For monomorphic functions (`type_params` empty), behavior is unchanged — falls
through to existing `is_compatible`.

### D5: `Unknown` interaction

`Unknown` remains compatible with everything, including `Var`. If an argument
type is `Unknown`, unification binds the corresponding `Var` to `Unknown`, and
the return type becomes `Unknown` after substitution. This preserves the
existing graceful-degradation behavior.

## Implementation phases

### Phase 1 — Core type representation (`typecheck-core`) ✅

**Files:**

- `crates/typecheck-core/src/type_.rs` — add `Type::Var(String)`, update `name()`
- `crates/typecheck-core/src/compat.rs` — handle `Var` in `is_compatible`:
  - `Var` vs `Var` (same name) → true
  - `Var` vs concrete → false (outside unification)
  - `Var` vs `Unknown`/`Never` → true (graceful degradation)
  - `Var` in nested positions (Array, Object, Function) → recurse normally
- `crates/typecheck-core/src/lib.rs` — re-export any new items

**Tests:** `crates/typecheck-core/src/lib_tests.rs`

- `Var` naming
- `Var` self-compatibility
- `Var` vs concrete (incompatible outside unification)
- `Var` vs `Unknown` (compatible)
- `Array<Var<T>>` vs `Array<String>` (incompatible outside unification)

### Phase 2 — Unification engine (`typecheck-core`) ✅

**New file:** `crates/typecheck-core/src/unify.rs`

```rust
/// A substitution map: type variable name → concrete type.
pub type Substitution = BTreeMap<String, Type>;

/// Unify `actual` against `pattern`, updating `subst`.
/// Returns `true` if unification succeeds.
///
/// - `Var(name)` in pattern: bind or check consistency
/// - `Unknown` / `Never`: compatible with anything
/// - Concrete types: structural recursion
/// - `Var` in actual but not pattern: compatible only if pattern is Unknown/Never
pub fn unify(actual: &Type, pattern: &Type, subst: &mut Substitution) -> bool

/// Apply a substitution to a type, replacing all bound Vars.
pub fn substitute(ty: &Type, subst: &Substitution) -> Type
```

**Tests:** `crates/typecheck-core/src/unify_tests.rs`

- Bind `T` to `String`
- Consistent rebinding (same `T`, same type)
- Inconsistent rebinding (same `T`, different types) → fail
- Nested: `Array<T>` unify `Array<String>` → `T=String`
- Function unify: `fn(T) -> T` unify `fn(String) -> String` → `T=String`
- `Unknown` in actual → binds `T` to `Unknown`
- `substitute` replaces all vars, leaves unbound vars as-is

### Phase 3 — `FunctionType` type params (`typecheck-core` + `typecheck-infer`) ✅

**Files:**

- `crates/typecheck-core/src/type_.rs` — add `type_params: Vec<String>` to `FunctionType`
- `crates/typecheck-infer/src/table.rs` — update `FunctionTable::from_signatures` and `RawFnSig` to carry type params
- `crates/typecheck-infer/src/lib.rs` — update `check_call`:
  - If `type_params` is empty → existing monomorphic path (no change)
  - If `type_params` is non-empty → create fresh `Substitution`, unify each arg against param type, substitute return type
- `crates/typecheck-infer/src/lib.rs` — update `infer_ast` Call case to use new `check_call`

**`RawFnSig` change:**

```rust
// Before:
pub type RawFnSig = (String, Vec<(String, JsonValue)>, Option<JsonValue>);
// After:
pub type RawFnSig = (String, Vec<String>, Vec<(String, JsonValue)>, Option<JsonValue>);
//                          ^type_params   ^params              ^return
```

**Tests:** `crates/typecheck-infer/src/lib_tests.rs`

- Monomorphic call still works (regression)
- Generic `identity<T>(x: T) -> T` called with `String` → infers `String`
- Generic `map<T, U>(arr: Array<T>, fn: (T) -> U) -> Array<U>` called with `Array<String>` and `fn(String) -> Number` → infers `Array<Number>`
- Generic call with `Unknown` arg → infers `Unknown`
- Type mismatch in generic call → infers `Unknown`

### Phase 4 — JSON Schema interpreter (`typecheck-schema-json`) ✅

**Files:**

- `crates/typecheck-schema-json/src/interpret.rs`:
  - `interpret_schema`: handle `"type": "var"` → read `"name"` field → `Type::Var(name)`
  - `interpret_schema`: handle `"type": "function"` → read `"typeParams"` array → pass to `FunctionType`
  - `interpret_function`: accept and propagate type params
  - `infer_json_type`: no change needed (concrete values can't be type vars)

**Tests:** `crates/typecheck-schema-json/src/lib_tests.rs`

- `{ type: var, name: T }` → `Type::Var("T")`
- `{ type: array, items: { type: var, name: T } }` → `Array(Var("T"))`
- Function with `typeParams` → `FunctionType` with populated `type_params`
- Function with no `typeParams` → empty `type_params` (regression)

### Phase 5 — Runner integration (`compiler-pass-typecheck`) ✅

**Files:**

- `crates/compiler-pass-typecheck/src/runner.rs`:
  - Extract function signatures from document data (currently `fn_sigs` is empty `Vec::new()`)
  - This is a pre-existing gap — the runner doesn't populate the function table from document data yet
  - Walk the `functions` field (if present) in extracted data, build `RawFnSig` entries with type params
  - Pass the populated table to `check()` calls

**Note:** This phase also fixes a pre-existing bug where the function table is always empty. The runner at `:20` creates `let fn_sigs: Vec<RawFnSig> = Vec::new()`. Populating it from document function declarations is needed for generics to be useful in practice, but is orthogonal — it benefits monomorphic functions too.

### Phase 6 — Documentation ✅

**Files:**

- `docs/LANG.md` §9 (Type system):
  - Add `Var(String)` to the type representation table
  - Document `typeParams` in function schema syntax
  - Add unification rules to §9.5 (Type compatibility)
  - Add examples of generic function declarations and call inference
- `docs/explanations/small-kernel.md`:
  - Note that generics are a typechecker feature, not a kernel primitive
  - Add `map`, `filter` generic signatures as examples in "emergent patterns"

## Dependency graph

```
Phase 1 (Type::Var)
  └── Phase 2 (unify.rs)
        └── Phase 3 (FunctionType + check_call)
              ├── Phase 4 (JSON Schema interpreter)
              └── Phase 5 (runner integration)
                    └── Phase 6 (docs)
```

Phases 1–3 are the core. Phases 4–5 make it usable end-to-end. Phase 6 is docs.

## What is NOT in scope

- **No runtime changes.** The VM, AST, host traits, value types, and
  continuation model are untouched. Generics are erased at runtime.
- **No generic user-defined functions in document syntax (yet).** Phase 5
  populates the function table from document data, but the schema syntax for
  declaring `typeParams` on user-defined functions is the same convention as
  host functions (D3). This works today — the schema interpreter just needs to
  read the field.
- **No higher-kinded types.** No `Type::Var` that itself takes type parameters.
- **No variance annotations.** All type params are invariant (like Rust's `T`
  in `Vec<T>`). This is the simplest correct default.
- **No trait bounds / constraints.** Type params are unbounded. A `T` can unify
  with any concrete type. Bounds can be added later if needed.
- **No type inference of type params from context.** Type params are inferred
  from argument types only (like Rust's inference from call arguments), not
  from expected return type.

## Risk assessment

- **Breaking changes:** Adding a field to `FunctionType` is a structural change.
  All construction sites need updating. There are ~5 construction sites across
  the codebase (table.rs, interpret.rs, test files). Low risk, mechanical.
- **Performance:** Unification is O(n) in the number of type variables per call.
  For AC's small function signatures (typically 1–3 params), this is negligible.
  The existing benchmark suite (`crates/bench`) will catch regressions.
- **Backward compatibility:** Monomorphic functions (empty `type_params`) follow
  the existing `is_compatible` path. No behavioral change for existing documents.
