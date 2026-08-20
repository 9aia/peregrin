# AC Language Technical Specification

**Version:** 0.1.0
**Status:** Draft

## 1. Overview

AC is a configuration and expression language built around three orthogonal axes:

- **Values** — JSON-compatible data represented by `RuntimeValue`.
- **Schemas** — abstract validation via a generic schema interface; JSON Schema is the default implementation.
- **Execution** — expression trees evaluated by a VM with host-provided functions.

A document can describe its own structure (`fields:`), declare function signatures (`functions:`), and interpolate values (`${...}`) — all validated by the same schema system.

The language also provides compile-time tooling: a **type system** for static type checking, a **type generation** pipeline that produces JSON Schema from field declarations, and a **module resolution** system for importing external documents. These are composed via a pluggable **engine pipeline** with passes for parsing, extraction, resolution, type checking, and generation.

### Design principles

1. **Core is dependency-free.** The core module defines expression trees and field bindings with zero external dependencies.
2. **Frontends are pluggable.** Any format that parses into `RuntimeValue` and serializes back can implement the frontend interface.
3. **Schemas are pluggable.** Any validation system can implement the schema interface. JSON Schema is one implementation, not the abstraction.
4. **Functions are host-provided.** The VM has no built-in functions. All capabilities come from the host.
5. **Interpolation is domain logic.** Field extraction, interpolation, and function call resolution live in the fields module, not CLI plumbing.
6. **AC keys are isolated.** Each AC-specific key (`expr`, `arg`, `merge`, `inheritable`, `mutable`) has its own crate for type checking and type generation. The core type system and typegen context have no knowledge of AC keys.
7. **Pipeline is composable.** The engine pipeline is a sequence of stateless passes. Custom passes (linting, optimization, JIT) slot in without core changes.

### Naming: "fields" not "keys"

The reserved `fields:` key declares typed, validated members of a document. The term "fields" is deliberate:

- **Fields imply schema.** A field has a name, a type, and constraints. "Keys" is generic — every object property in JSON/YAML is already a key. Overloading `keys:` to mean "schema declarations" makes a common term ambiguous.
- **Self-reference breaks.** Today `fields:` is a reserved key that cannot be declared as a field name. Renaming to `keys:` yields "keys is a reserved key that cannot be declared as a key name" — the sentence collapses.
- **Convention.** Schema languages (protobuf, Avro, SQL, JSON Schema's "properties") use "fields" or "properties" for typed, declared members. "Keys" is the substrate — the raw object entries in a key-value store. ac-lang is in the schema camp.
- **`Field<V>` is foundational.** The core type `Field<V>` is a named binding of a value. "Key" is an identifier; "field" is a name + value + schema.

## 2. Architecture

The workspace is organized into five layers, each building on the one below. No layer depends on a layer above it.

```
Layer 5 — CLI                     ac-cli
Layer 4 — Engine pipeline         ac-engine, ac-engine-core, ac-engine-pass-*
Layer 3 — Type system & generation ac-typecheck-*, ac-typegen-*, ac-resolver-*
Layer 2 — Domain logic            ac-fields, ac-host, ac-frontend
Layer 1 — Core                    ac-core, ac-runtime, ac-vm, ac-schema, ac-schema-json, ac-frontend-*
```

### Crate inventory

#### Layer 1 — Core

| Crate             | Responsibility                                               | Dependencies    |
| ----------------- | ------------------------------------------------------------ | --------------- |
| **core**          | `Expr<V>`, `Field<V>` — generic, zero external dependencies  | None            |
| **runtime**       | `RuntimeValue` (wraps `serde_json::Value`), `Frontend` trait | core            |
| **vm**            | `Vm<H>`, `Host`, `Function`, `Context`, `Error`              | core, runtime   |
| **schema**        | `Schema<V>` trait, `ValidationError` — zero dependencies     | None            |
| **schema-json**   | `JsonSchema` — JSON Schema implementation of `Schema<V>`     | schema, runtime |
| **frontend-json** | JSON parse/serialize → `RuntimeValue`                        | runtime         |
| **frontend-yaml** | YAML parse/serialize → `RuntimeValue`                        | runtime         |
| **frontend-toml** | TOML parse/serialize → `RuntimeValue`                        | runtime         |

#### Layer 2 — Domain logic

| Crate        | Responsibility                                                             | Dependencies                     |
| ------------ | -------------------------------------------------------------------------- | -------------------------------- |
| **fields**   | Field extraction, interpolation, computed fields, merge, scope, validation | core, runtime, schema            |
| **host**     | `DefaultHost` with `Add`, `Mul`, `Concat`, `Len` primitives                | runtime, vm                      |
| **frontend** | `frontend_for()` factory — maps format names to `Frontend` impls           | runtime, frontend-json/yaml/toml |

#### Layer 3 — Type system, type generation, module resolution

| Crate                           | Responsibility                                                                       | Dependencies                                        |
| ------------------------------- | ------------------------------------------------------------------------------------ | --------------------------------------------------- |
| **typecheck-core**              | `Type`, `FunctionType`, `TypeEnv`, `TypeError`, `SchemaInterpreter`, `is_compatible` | None (uses serde_json)                              |
| **typecheck-infer**             | Expression type inference: `infer()`, `FunctionTable`, `ExprAst`, `parse_expr()`     | typecheck-core                                      |
| **typecheck-schema-json**       | `JsonSchemaInterpreter` — JSON Schema → `Type` mapping                               | typecheck-core                                      |
| **typecheck-key-expr**          | Typecheck `expr` key — inferred type vs declared type                                | typecheck-core, typecheck-infer                     |
| **typecheck-key-arg**           | Typecheck `arg` key — arg type vs declared type                                      | typecheck-core                                      |
| **typecheck-key-merge**         | Typecheck `merge` key — merge fields must be objects, props compatible               | typecheck-core                                      |
| **typecheck-key-inherit**       | Typecheck `inheritable` key — override types compatible with parent                  | typecheck-core                                      |
| **typecheck-key-mutable**       | Typecheck `mutable` key — immutable fields not overridden                            | typecheck-core                                      |
| **typecheck-import**            | Cross-module type checking via `ModuleResolver`                                      | typecheck-core, resolver-core                       |
| **typegen-core**                | `TypegenContext`, `TypegenBackend` trait, `strip_keys`                               | None (uses serde_json, indexmap)                    |
| **typegen-key-merge**           | Filter merge fields from output, strip `merge` key                                   | typegen-core                                        |
| **typegen-key-inherit**         | Strip `inheritable` key from schemas                                                 | typegen-core                                        |
| **typegen-key-mutable**         | Strip `mutable` key from schemas                                                     | typegen-core                                        |
| **typegen-key-expr**            | Strip `expr` key, read expr values for import detection                              | typegen-core                                        |
| **typegen-key-arg**             | Strip `arg` key from schemas                                                         | typegen-core                                        |
| **typegen-import**              | Follow `import()` expressions, merge imported schemas into context                   | typegen-core, typegen-key-merge/expr, resolver-core |
| **typegen-backend-json-schema** | `JsonSchemaBackend` — produces JSON Schema (draft-07) from clean fragments           | typegen-core                                        |
| **resolver-core**               | `ModuleResolver` trait, `ResolvedModule`, `CachingResolver` wrapper                  | None (uses serde_json)                              |
| **resolver-file**               | `FileResolver` — filesystem-based module loading                                     | resolver-core, fields, frontend-*                   |
| **resolver-virtual**            | `VirtualResolver` — in-memory module loading (for tests)                             | resolver-core, fields, frontend-yaml                |

#### Layer 4 — Engine pipeline

| Crate                     | Responsibility                                                                                 | Dependencies                                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| **engine-core**           | `Compilation`, `Source`, `Config`, `Pass` trait, `Pipeline` builder                            | fields, runtime, typecheck-core, typegen-core                                                                   |
| **engine-pass-parse**     | `ParsePass` — parse source text into `RuntimeValue`                                            | engine-core, frontend                                                                                           |
| **engine-pass-extract**   | `ExtractPass` — extract fields, functions, data from parsed document                           | engine-core, fields                                                                                             |
| **engine-pass-resolve**   | `ResolvePass` — resolve imports, build `TypegenContext`                                        | engine-core, typegen-core, typegen-key-*, typegen-import                                                        |
| **engine-pass-typecheck** | `TypecheckPass` — typecheck all AC keys and cross-module imports                               | engine-core, typecheck-core, typecheck-*, resolver-core                                                         |
| **engine-pass-generate**  | `GeneratePass` — generate output from resolved `TypegenContext`                                | engine-core, typegen-core                                                                                       |
| **engine**                | Preset pipelines + non-pipeline operations (parse, eval, validate, fields, typegen, typecheck) | engine-core, all pass crates, host, frontend, resolver-file, typecheck-schema-json, typegen-backend-json-schema |
| **engine-tests**          | Integration tests for the full pipeline                                                        | engine                                                                                                          |

#### Layer 5 — CLI

| Crate   | Responsibility                                    | Dependencies |
| ------- | ------------------------------------------------- | ------------ |
| **cli** | `ac` CLI binary — thin wrapper around `ac-engine` | engine, clap |

### Dependency constraints

- **core** and **schema** have zero external dependencies.
- **runtime** is the only crate that couples to a JSON value library for values.
- **fields** does not depend on **vm** — the caller bridges the two via a function resolver interface.
- **typecheck-core** and **typegen-core** have no knowledge of AC-specific keys (`expr`, `arg`, `merge`, `inheritable`, `mutable`). Each key has its own crate.
- **resolver-core** defines the `ModuleResolver` trait; implementations (`resolver-file`, `resolver-virtual`) do the I/O.
- **engine-core** is pass-agnostic — it defines the `Pass` trait and `Pipeline` builder but depends on no pass crates.
- **engine** depends on all pass crates and provides preset pipelines.
- **cli** depends only on **engine** — all logic lives in the engine.

## 3. Core types

### 3.1 Expression tree

An expression tree is the sole description of computation. It is generic over the value type `V`.

| Variant   | Meaning                                                               |
| --------- | --------------------------------------------------------------------- |
| **Value** | A literal value — no further evaluation needed.                       |
| **Call**  | A function invocation with a name and a list of argument expressions. |

The VM evaluates `Call` arguments left-to-right, then dispatches to the host for execution.

### 3.2 Field binding

A field is a named binding of a value. Fields are the building blocks of objects — an object is conceptually a collection of fields.

## 4. Runtime values

### 4.1 `RuntimeValue`

The concrete value type used throughout the system. It wraps a JSON-compatible value, supporting the same set of types: null, boolean, number, string, array, and object.

The type provides:

- **Predicates** to test the kind of a value (null, bool, number, string, array, object).
- **Accessors** to read typed values (boolean, number, string, array, object elements/properties).
- **Conversions** from and to JSON values, and from primitive types (bool, number, string).

### 4.2 `HostValue`

An alias for `RuntimeValue` used at the VM/host boundary to clarify intent.

## 5. Virtual machine

### 5.1 Execution engine

The VM walks expression trees and produces values. It is parameterized by a host that provides functions.

**Evaluation rules:**

1. A `Value` node evaluates to itself.
2. A `Call` node evaluates each argument left-to-right, resolves the function by name via the host, then invokes the function with the evaluated arguments and the current context.

### 5.2 Host interface

The host provides function lookups by name. The VM has no built-in functions — all capabilities (`add`, `mul`, `concat`, `len`, etc.) are provided by the host implementation.

### 5.3 Function interface

A function receives evaluated arguments and the execution context, and returns a value or an error. Functions must be callable without exclusive access to themselves; functions requiring mutable state must use interior mutability patterns.

### 5.4 Execution context

The context maintains a stack of scopes for variable bindings.

| Operation                  | Behavior                                                                           |
| -------------------------- | ---------------------------------------------------------------------------------- |
| **Create**                 | Initializes with one empty global scope.                                           |
| **Push scope / Pop scope** | Model function call boundaries. The global scope is never popped.                  |
| **Define**                 | Sets or overwrites a variable in the current scope.                                |
| **Resolve**                | Searches from the innermost scope outward. Returns `UnknownVariable` if not found. |
| **Depth**                  | Reports the current scope depth (global = 1).                                      |

### 5.5 Errors

| Error             | Meaning                                                   |
| ----------------- | --------------------------------------------------------- |
| `UnknownFunction` | A function name was not found in the host.                |
| `UnknownVariable` | A variable name was not found in any scope.               |
| `PrimitiveError`  | A primitive operation failed with a message.              |
| `TypeMismatch`    | A value did not match the expected type.                  |
| `ArgumentCount`   | The number of arguments did not match the expected count. |

Errors are comparable for testing and replay purposes.

## 6. Schema system

### 6.1 Schema interface

A schema validates a value of type `V`, returning a validation error if the value does not conform. The interface is generic over `V` and independent of any specific schema language.

### 6.2 Validation error

A validation error carries:

- **message** — human-readable description of the failure.
- **path** — dotted path to the offending element within the value (empty for root).

### 6.3 JSON Schema

The default schema implementation validates `RuntimeValue` against a JSON Schema document.

**Supported keywords:**

| Keyword    | Description                                                                 |
| ---------- | --------------------------------------------------------------------------- |
| `type`     | One of: `string`, `number`, `integer`, `boolean`, `null`, `array`, `object` |
| `required` | Array of required property names (objects only)                             |

This is a minimal validator. It covers the most common cases. Replace with a full JSON Schema validator when needed — the schema interface stays the same.

## 7. Frontends

### 7.1 Frontend interface

A frontend parses a text input into a `RuntimeValue` and serializes a `RuntimeValue` back to text. The interface is defined in the runtime module; each frontend crate provides one implementation.

### 7.2 Built-in frontends

| Crate           | Format |
| --------------- | ------ |
| `frontend-json` | JSON   |
| `frontend-yaml` | YAML   |
| `frontend-toml` | TOML   |

### 7.3 Frontend factory

The `frontend` crate provides a `frontend_for(format)` function that returns a boxed `Frontend` for the requested format name. Supported formats: `yaml`/`yml`, `json`, `toml`. This crate is interface-agnostic — usable from CLI, MCP, or any consumer that needs to parse/serialize AC documents.

Adding a new format (XML, INI, etc.) requires only implementing the frontend interface and adding a match arm to `frontend_for`. The core, VM, and schema system are unaffected.

## 8. Fields system

The fields module provides domain logic for self-describing documents.

### 8.1 Document structure

A document is a `RuntimeValue` (typically an object) that may contain two reserved top-level keys:

- **`fields:`** — declares field schemas for the document's data fields.
- **`functions:`** — declares function signatures (param and return schemas).

Both keys are extracted and removed from the data before interpolation and validation.

**Reserved keys:** `fields` and `functions` cannot be declared as field names within `fields:`.

### 8.2 Field extraction

Extracting fields separates a document into three components:

- **Schemas** — field name to raw schema mapping, from the `fields:` section.
- **Functions** — function name to signature mapping, from the `functions:` section.
- **Data** — the document with `fields:` and `functions:` removed.

Returns an error if a declared field name is reserved (`fields` or `functions`).

### 8.3 Function signatures

A function signature declares:

- **Params** — parameter name to raw schema mapping.
- **Return schema** — return type schema, if declared.
- **Body** — expression body string, if user-defined.

Declared in the `functions:` section:

```yaml
functions:
  add:
    params:
      x: { type: number }
      y: { type: number }
    return: { type: number }
```

Functions with a `body:` field are **user-defined** — evaluated by parsing the body as an expression. Functions without a `body:` field are **host primitives** — resolved by the `FunctionResolver` (typically dispatching to the VM).

```yaml
functions:
  double:
    params:
      x: { type: number }
    return: { type: number }
    body: "mul(x, 2)"
```

### 8.3.1 User-defined function bodies

A `body:` expression is evaluated as follows:

1. Params are merged into the `doc` namespace (shadowing doc fields with the same name).
2. The body expression is evaluated as an expression string.
3. Bare names like `x` resolve via shorthand path lookup (→ `doc.x`).
4. The resolver is passed through, so the body can call host primitives and other user-defined functions.

**Body syntax** — same as `${...}` content (without the `${}` envelope):

| Form            | Example             | Resolves to                             |
| --------------- | ------------------- | --------------------------------------- |
| Function call   | `mul(x, 2)`         | Host primitive or user-defined function |
| Nested call     | `double(double(x))` | Recursive composition                   |
| Path lookup     | `doc.version`       | Namespace path                          |
| Shorthand       | `x`                 | `doc.x` (param)                         |
| String literal  | `"hello"`           | String                                  |
| Number literal  | `42`                | Number                                  |
| Boolean literal | `true`              | Boolean                                 |

**Recursion safety:** A depth limit (64) prevents infinite self-reference.

**Expression evaluation** — a public API evaluates a single expression string against the current namespaces and resolver.

### 8.4 Interpolation

Interpolation recursively replaces `${...}` patterns in string values. Recursion covers objects, arrays, and nested strings. It takes a value, the current namespaces, and a function resolver.

#### Interpolation syntax

| Syntax                 | Kind                          | Example                      |
| ---------------------- | ----------------------------- | ---------------------------- |
| `${ns.key}`            | Namespace path                | `${doc.name}`, `${env.HOME}` |
| `${ns.key.sub}`        | Nested path                   | `${doc.author.email}`        |
| `${key}`               | Shorthand path                | `${name}` → `doc.name`       |
| `${fn()}`              | Function call (no args)       | `${now()}`                   |
| `${fn(args)}`          | Function call                 | `${add(1, 2)}`               |
| `${fn(ns.key, "lit")}` | Function call with mixed args | `${concat(doc.name, "!")}`   |
| `${fn(fn(a, b), c)}`   | Nested function calls         | `${add(mul(2, 3), 4)}`       |

**Discriminator:** `()` distinguishes function calls from path lookups. If the expression contains `(`, it's a function call. Otherwise, it's a path lookup.

**Shorthand:** If the first segment of a path is not a known namespace, the resolver tries `doc` as the namespace. `${name}` is equivalent to `${doc.name}`.

#### Type preservation

When the entire string value is a single `${...}` expression, the raw value is returned (preserving type). When the string contains other text, all `${...}` patterns are stringified and concatenated.

| Value                          | Result            | Type   |
| ------------------------------ | ----------------- | ------ |
| `"${mul(version, 2)}"`         | `2`               | number |
| `"result: ${mul(version, 2)}"` | `"result: 2"`     | string |
| `"hello, ${doc.name}!"`        | `"hello, alice!"` | string |

#### Argument evaluation

Each function argument is evaluated independently:

| Argument form              | Resolves to                          |
| -------------------------- | ------------------------------------ |
| `"literal"` or `'literal'` | String literal                       |
| `123` or `1.5`             | Number literal                       |
| `true` / `false`           | Boolean literal                      |
| `null`                     | Null                                 |
| `${...}`                   | Nested interpolation (recursive)     |
| `ns.key` or `key`          | Path lookup (namespace or shorthand) |
| `fn(args)`                 | Nested function call                 |

#### Namespaces

Namespaces are a map of name to JSON value.

| Namespace | Source                        | Example         |
| --------- | ----------------------------- | --------------- |
| `doc`     | Document's own data fields    | `${doc.name}`   |
| `env`     | Process environment variables | `${env.HOME}`   |
| custom    | Added by the caller           | `${custom.key}` |

**Built-in namespace builders:**

- **doc** — constructed from the document's data fields.
- **env** — constructed from process environment variables.

### 8.5 Function resolver interface

Bridges interpolation to the VM (or any other execution engine). The fields module does not depend on the VM module — the caller provides a function resolver implementation.

A function resolver receives a function name and evaluated arguments, and returns a value or an error.

The CLI's resolver performs the following steps:

1. Looks up the function's declared signature (if any).
2. Validates args against param schemas.
3. If the function has a `body:` — evaluates it (user-defined).
4. If no `body:` — dispatches the call via the VM (host primitive).
5. Validates the return value against the return schema.
6. Enforces a recursion depth limit (64) to prevent infinite self-reference.

A no-op resolver that fails all calls is available for use when no functions are needed.

### 8.6 Typed parameter validation

Function arguments and return values are validated using the schema system — the same system used for field validation. This means function params and return values are validated by the same schema language as fields.

### 8.7 Undeclared fields

Returns names of fields present in the data but not declared in `fields:`.

### 8.8 Scoped inheritance

Fields declared `inheritable: true` propagate their values into nested objects. As scoped interpolation descends into objects, inheritable fields are merged into the `doc` namespace. Child objects inherit the parent's scope and can override inheritable fields locally — the override applies to that subtree only.

If no fields are inheritable, scoped interpolation delegates to standard interpolation — no scope tracking overhead.

**How it works:**

1. At each object level, collect local inheritable field values.
2. Merge with parent scope (local overrides win).
3. Inject merged scope into the `doc` namespace for children.
4. Recurse with updated scope and namespace.

**Example:**

```yaml
fields:
  permission_mode:
    type: string
    inheritable: true
permission_mode: "read-only"
config:
  message: "${permission_mode}" # → "read-only"
  server:
    permission_mode: "read-write"
    message: "${permission_mode}" # → "read-write"
  deep:
    message: "${permission_mode}" # → "read-only"
```

No new syntax — `${permission_mode}` is standard shorthand. The scope is transparent to the user.

### 8.9 Immutability validation

Fields declared `mutable: false` cannot be overridden in nested scopes.

Validation walks the data tree. For each immutable field, it compares every nested occurrence against the root value. Returns error messages for any mismatches.

The extracted fields result also includes:

- **Inheritable** — list of field names with `inheritable: true`.
- **Immutable** — list of field names with `mutable: false`.
- **Computed** — list of `(field_name, expr_string)` pairs with `expr:` key.

Both are extracted from the raw schema values during field extraction. The caller passes them to scoped interpolation and immutability validation respectively.

### 8.10 Computed fields

Fields declared with an `expr:` key are computed after interpolation. The expression uses the same syntax as `${...}` content: function calls, path lookups (via namespaces), and literals.

Computation evaluates each field's expression and inserts the result into the data. The `doc` namespace is updated with the data before each pass so expressions see current field values. Multiple passes resolve dependencies between computed fields — if field A depends on field B, B is computed first and A resolves in the next pass. If no progress is made in a pass (circular or unresolvable dependencies), errors are reported for the remaining fields.

If a computed field is also present in the data, the `expr:` value takes precedence.

## 9. Type system

The type system provides static type checking of AC documents before runtime evaluation. It is schema-system-agnostic — the `SchemaInterpreter` trait converts any schema language's vocabulary into a neutral `Type` enum.

### 9.1 Type representation

| Variant    | Meaning                                                      |
| ---------- | ------------------------------------------------------------ |
| `String`   | String type                                                  |
| `Number`   | Number type (includes integers — no integer subtyping yet)   |
| `Boolean`  | Boolean type                                                 |
| `Null`     | Null type                                                    |
| `Array(T)` | Array with element type `T`                                  |
| `Object`   | Object with named property types (`BTreeMap<String, Type>`)  |
| `Function` | Function type — parameter names/types and return type        |
| `Unknown`  | Cannot be statically determined — compatible with everything |
| `Never`    | Impossible code path — compatible with everything            |

`Unknown` represents types that cannot be statically determined (e.g., an unresolvable path or unknown function). `Never` represents uninhabited code paths. Both are compatible with all other types.

### 9.2 Type environment

`TypeEnv` maps field names to their declared types. It supports scope chaining for inheritance: a child scope inherits all bindings from its parent and can override individual fields. Lookups walk the chain from child to parent.

### 9.3 Schema interpreter

The `SchemaInterpreter` trait converts a raw schema fragment into a `Type`. Each schema language implements this trait:

- `JsonSchemaInterpreter` (in `typecheck-schema-json`) — maps JSON Schema `type` keywords to `Type` values. Handles `string`, `number`/`integer`, `boolean`, `null`, `array` (with `items`), `object` (with `properties`), and `enum` (infers type from first enum value).

### 9.4 Expression type inference

The `typecheck-infer` crate parses expression strings (same syntax as `${...}` and `expr:` fields) and infers their static type given a `TypeEnv` and `FunctionTable`.

**Expression AST:**

| Variant   | Form               | Example                                    |
| --------- | ------------------ | ------------------------------------------ |
| `Literal` | Typed literal      | `"hello"`, `42`, `true`, `null`            |
| `Path`    | Dotted path lookup | `doc.name`, `env.HOME`, `name` (shorthand) |
| `Call`    | Function call      | `add(1, 2)`, `mul(x, 2)`                   |

Inference walks the AST:

- **Literal** → the literal's type.
- **Path** → looks up the first segment in `TypeEnv`; if not found, tries `doc` namespace shorthand. Walks remaining segments into object types.
- **Call** → looks up the function in `FunctionTable`, infers argument types, checks compatibility with param types, and returns the function's return type. Returns `Unknown` if the function is not found or argument counts mismatch.

A `FunctionTable` maps function names to `FunctionType`s, built from declared function signatures via the `SchemaInterpreter`.

### 9.5 Type compatibility

`is_compatible(actual, expected)` checks whether `actual` can be used where `expected` is required:

- `Unknown` is compatible with anything (both directions).
- `Never` is compatible with anything (both directions).
- Exact primitive matches are compatible.
- `Array<T>` is compatible if item types are compatible.
- `Object` is compatible if every expected property exists with a compatible type. Extra properties in `actual` are allowed.
- `Function` is compatible if param count matches and all param/return types are compatible.

### 9.6 Per-key type checking

Each AC-specific key has its own typecheck crate. The `TypecheckPass` orchestrator composes them:

| Crate                   | Key           | Check                                                              |
| ----------------------- | ------------- | ------------------------------------------------------------------ |
| `typecheck-key-expr`    | `expr`        | Inferred expression type must be compatible with declared type     |
| `typecheck-key-arg`     | `arg`         | Arg type must be compatible with field's declared type             |
| `typecheck-key-merge`   | `merge`       | Merge fields must be `object`; merged props compatible with parent |
| `typecheck-key-inherit` | `inheritable` | Override values in nested data must be type-compatible with root   |
| `typecheck-key-mutable` | `mutable`     | Immutable fields (`mutable: false`) must not be overridden         |
| `typecheck-import`      | `import()`    | Imported field types must be compatible with parent scope          |

All checks return `Vec<TypeError>`. Errors carry the field name, expected/actual types (if known), a human-readable message, and a dotted path to the location within the document.

### 9.7 Type errors

A `TypeError` carries:

- **field** — the field name where the error was detected.
- **expected** — the expected type, if known.
- **actual** — the actual (inferred) type, if known.
- **message** — human-readable explanation.
- **path** — dotted path to the location within the document (e.g., `config.server`).

## 10. Type generation

Type generation produces clean schema output from field declarations. It strips AC-specific keys (`expr`, `arg`, `merge`, `inheritable`, `mutable`), resolves imports, and emits a schema document via a pluggable backend.

### 10.1 Typegen context

`TypegenContext` holds field name → schema fragment pairs. It is built from raw schemas and then processed by AC-key crates:

1. **Filter merge fields** — merge fields are excluded from output (they're spread and removed at runtime).
2. **Strip AC keys** — `expr`, `arg`, `merge`, `inheritable`, `mutable` are removed from all field schemas.
3. **Merge imports** — imported module schemas are merged in (parent wins — existing fields are not overwritten).

### 10.2 Per-key typegen crates

Each AC key has a typegen crate that either filters or strips the key:

| Crate                 | Key           | Action                                                      |
| --------------------- | ------------- | ----------------------------------------------------------- |
| `typegen-key-merge`   | `merge`       | Filter merge fields from context; strip `merge` key         |
| `typegen-key-inherit` | `inheritable` | Strip `inheritable` key from all schemas                    |
| `typegen-key-mutable` | `mutable`     | Strip `mutable` key from all schemas                        |
| `typegen-key-expr`    | `expr`        | Strip `expr` key; provide `get_expr()` for import detection |
| `typegen-key-arg`     | `arg`         | Strip `arg` key from all schemas                            |

### 10.3 Import resolution for typegen

The `typegen-import` crate follows `import()` expressions in merge fields, resolves them via a `ModuleResolver`, and merges imported schemas into the parent context. It detects both static paths (`import('path')`) and data-driven paths (`import(arg)`). Imported schemas are filtered (merge fields removed) and stripped (AC keys removed) before merging.

### 10.4 Typegen backend

The `TypegenBackend` trait produces output from a `TypegenContext`:

| Implementation      | Output      | Extension      |
| ------------------- | ----------- | -------------- |
| `JsonSchemaBackend` | JSON Schema | `.schema.json` |

The backend also provides an IDE hint comment (e.g., `# yaml-language-server: $schema=<path>`) for editor integration and a file extension for emitted files.

Future backends: TypeScript `.d.ts`, Rust structs, Python `TypedDict`.

## 11. Module resolution

Module resolution abstracts how `import()` paths are loaded. The `ModuleResolver` trait in `resolver-core` defines the interface; implementations do the I/O.

### 11.1 Module resolver interface

A `ModuleResolver` resolves a module path (relative to an optional base path) and returns a `ResolvedModule` containing:

- **schemas** — field name → raw schema fragment pairs (AC keys still present).
- **merge** — field names marked `merge: true` in the imported module.
- **data** — the document's data values.
- **base_path** — base path for resolving nested imports from this module.

### 11.2 Caching resolver

`CachingResolver<R>` wraps any `ModuleResolver` with memoization. Subsequent calls with the same `(path, base)` key return a cached clone without re-invoking the inner resolver. This eliminates redundant disk I/O, parsing, and field extraction for repeated imports — including when `ResolvePass` and `TypecheckPass` both resolve the same import graph.

### 11.3 Implementations

| Crate              | Implementation    | I/O                         |
| ------------------ | ----------------- | --------------------------- |
| `resolver-file`    | `FileResolver`    | `std::fs` — reads from disk |
| `resolver-virtual` | `VirtualResolver` | In-memory `HashMap`         |

`FileResolver` auto-detects format by file extension (YAML/JSON/TOML), parses, and extracts fields. `VirtualResolver` stores pre-parsed documents in memory — ideal for tests and embedded scenarios.

Future implementations: `resolver-git`, `resolver-http`.

## 12. Engine pipeline

The engine provides a composable pipeline for compile-time operations: parsing, field extraction, import resolution, type checking, and type generation. The pipeline is a sequence of stateless `Pass`es that run in order, each receiving `&mut Compilation` and `&Config`.

### 12.1 Compilation

A `Compilation` flows through the pipeline, accumulating state as each pass runs:

| Field         | Populated by     | Type                             |
| ------------- | ---------------- | -------------------------------- |
| `source`      | Created at start | `Source` (input + format + path) |
| `parsed`      | `ParsePass`      | `Option<RuntimeValue>`           |
| `extracted`   | `ExtractPass`    | `Option<ExtractedFields>`        |
| `typegen_ctx` | `ResolvePass`    | `Option<TypegenContext>`         |
| `type_errors` | `TypecheckPass`  | `Vec<TypeError>`                 |
| `output`      | `GeneratePass`   | `Option<JsonValue>`              |

### 12.2 Configuration

`Config` holds the pluggable components as trait objects:

- **resolver** — `Box<dyn ModuleResolver>` — how modules are loaded.
- **interpreter** — `Box<dyn SchemaInterpreter>` — how schema fragments become types.
- **backend** — `Option<Box<dyn TypegenBackend>>` — how types are emitted (optional for typecheck-only runs).

The `engine` crate provides default configurations:

- `default_cli_config()` — `FileResolver` + `JsonSchemaInterpreter` + `JsonSchemaBackend`.
- `typecheck_only_config()` — `FileResolver` + `JsonSchemaInterpreter` + no backend.

### 12.3 Pass trait

Each pass implements `Pass`:

```rust
pub trait Pass {
    fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), String>;
    fn should_run(&self, _compilation: &Compilation, _config: &Config) -> bool { true }
}
```

`should_run` allows passes to skip when their preconditions are not met (e.g., skip `TypecheckPass` when no `expr` or `arg` keys exist). Custom passes (linting, optimization, JIT) implement `Pass` and slot into the pipeline without core changes.

### 12.4 Built-in passes

| Pass            | Crate                   | Input → Output                              |
| --------------- | ----------------------- | ------------------------------------------- |
| `ParsePass`     | `engine-pass-parse`     | source text → `parsed: RuntimeValue`        |
| `ExtractPass`   | `engine-pass-extract`   | `parsed` → `extracted: ExtractedFields`     |
| `ResolvePass`   | `engine-pass-resolve`   | `extracted` → `typegen_ctx: TypegenContext` |
| `TypecheckPass` | `engine-pass-typecheck` | `extracted` → `type_errors: Vec<TypeError>` |
| `GeneratePass`  | `engine-pass-generate`  | `typegen_ctx` → `output: JsonValue`         |

### 12.5 Preset pipelines

The `engine` crate provides preset pipelines:

| Preset      | Passes                                           | Use case                           |
| ----------- | ------------------------------------------------ | ---------------------------------- |
| `typegen`   | parse → extract → resolve → generate             | Generate JSON Schema from fields   |
| `typecheck` | parse → extract → typecheck                      | Static type checking               |
| `full`      | parse → extract → resolve → typecheck → generate | Type safety before code generation |

### 12.6 Non-pipeline operations

`parse`, `eval`, `validate`, and `fields` have their own entry points in the `engine` crate — they don't need the pipeline. These are runtime operations that use the VM, host, and fields module directly.

## 13. CLI

### 13.1 Commands

```
ac parse      --input <file> --format <yaml|json|toml>
ac eval       --input <file> --format <yaml|json|toml> --output <yaml|json|toml>
ac validate   --input <file> --format <yaml|json|toml> --schema <schema.json>
ac fields     --input <file> --format <yaml|json|toml>
ac typegen    --input <file> --format <yaml|json|toml> [--stdout] [--output-dir <dir>]
ac typecheck  --input <file> --format <yaml|json|toml>
```

If `--input` is omitted, reads from stdin (except `typegen` which requires `--input`).

### 13.2 `parse`

Parses input in the specified format, outputs as YAML.

### 13.3 `eval`

Parses input, evaluates as an expression tree, outputs the result in the specified output format. If the input is an object, each key is defined as a variable in the execution context.

### 13.4 `validate`

Parses input, validates against an external JSON Schema file. Outputs `valid` or `invalid: <error>`.

### 13.5 `fields`

Processes a self-describing document:

1. **Extract** `fields:` and `functions:` sections.
2. **Build** namespaces (`doc` from data, `env` from environment).
3. **Interpolate** `${...}` patterns in all data values (path lookups + function calls).
4. **Compute** fields with `expr:` key (after interpolation, multiple passes for dependencies).
5. **Merge** fields with `merge: true` (spread computed object into parent, remove field).
6. **Validate** each field against its declared schema.
7. **Report** undeclared fields and immutability violations.

### 13.6 `typegen`

Generates type definitions from `fields:` declarations. Runs the typegen pipeline (parse → extract → resolve → generate) and writes JSON Schema output.

Options:

- `--stdout` — print schema to stdout instead of writing files.
- `--output-dir <dir>` — output directory for generated schema files (default: `.ac/generated/schemas`).

When writing to disk, also injects an IDE hint comment into the source file (e.g., `# yaml-language-server: $schema=<path>`) for editor integration.

### 13.7 `typecheck`

Static type checking of `fields:` declarations. Runs the typecheck pipeline (parse → extract → typecheck) and reports any type errors. Outputs `ok` or a list of type errors with field names, expected/actual types, and paths.

### 13.8 Built-in host functions

The CLI provides these functions via its default host:

| Function | Args                        | Returns   | Description                                                                                                                                                                                                                      |
| -------- | --------------------------- | --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `add`    | `number...`                 | `number`  | Sum of all numeric args                                                                                                                                                                                                          |
| `mul`    | `number...`                 | `number`  | Product of all numeric args                                                                                                                                                                                                      |
| `concat` | `string...`                 | `string`  | Concatenation of all string args                                                                                                                                                                                                 |
| `len`    | `string \| array \| object` | `integer` | Length of the argument                                                                                                                                                                                                           |
| `import` | `string`                    | `object`  | Loads an external document by path, extracts its data (strips `fields:`/`functions:`), and returns it. Paths resolve relative to the input file. Results are cached by resolved path. Format is auto-detected by file extension. |

## 14. Processing pipeline

### 14.1 Runtime pipeline (`ac fields`)

```
Input file (YAML/JSON/TOML)
     │
     ▼
  Parse via frontend → RuntimeValue
     │
     ▼
  Extract fields → schemas, functions, data
     │
     ▼
  Build namespaces { doc: data, env: env_vars, ... }
     │
     ▼
  Interpolate data (path lookups + function calls)
     │  ├── Path lookups: resolve via namespaces
     │  └── Function calls: resolve via function resolver
     │       ├── Validate args against param schemas
     │       ├── Dispatch via VM + host
     │       └── Validate return against return schema
     │
     ▼
  Compute fields (for each field with expr:)
     │  └── Multiple passes resolve dependencies
     │       └── Fields not in data are inserted
     │       └── If field has arg:, field's data value
     │          is available as ${arg} in its expr
     │
     ▼
  Merge fields (for each field with merge: true)
     │  └── Spread computed object value into parent scope
     │  └── Remove the field itself from data
     │  └── Parent wins: existing keys are not overwritten
     │
     ▼
  Validate each field against its schema
     │
     ▼
  Report undeclared fields
     │
     ▼
  Output: all fields valid / N validation error(s)
```

### 14.2 Compile-time pipeline (`ac typegen` / `ac typecheck`)

```
Input file (YAML/JSON/TOML)
     │
     ▼
  ParsePass → parsed: RuntimeValue
     │
     ▼
  ExtractPass → extracted: ExtractedFields
     │
     ├── Typegen path:
     │      ▼
     │   ResolvePass → typegen_ctx: TypegenContext
     │      │  ├── Filter merge fields
     │      │  ├── Strip AC keys (expr, arg, merge, inheritable, mutable)
     │      │  └── Resolve imports via ModuleResolver
     │      ▼
     │   GeneratePass → output: JsonValue (JSON Schema)
     │
     └── Typecheck path:
            ▼
         TypecheckPass → type_errors: Vec<TypeError>
            │  ├── Check expr key types (inferred vs declared)
            │  ├── Check arg key types (arg vs declared)
            │  ├── Check merge key types (must be object, props compatible)
            │  ├── Check inheritable key types (overrides compatible)
            │  ├── Check mutable key (immutable not overridden)
            │  └── Check import types (cross-module compatibility)
            ▼
         Output: ok / N type error(s)
```

## 15. Extension points

| Extension                | How                                                        | Touches            |
| ------------------------ | ---------------------------------------------------------- | ------------------ |
| New format               | Implement the frontend interface + add to `frontend_for()` | New crate only     |
| New schema language      | Implement the `Schema<V>` interface                        | New crate only     |
| New schema interpreter   | Implement `SchemaInterpreter` for type checking            | New crate only     |
| New typegen backend      | Implement `TypegenBackend` for type generation             | New crate only     |
| New module resolver      | Implement `ModuleResolver`                                 | New crate only     |
| New host function        | Implement `Function`, register with host                   | Host impl only     |
| Custom namespace         | Insert into the namespaces map                             | Caller code        |
| Custom function resolver | Implement `FunctionResolver`                               | Caller code        |
| Custom pipeline pass     | Implement `Pass`, add to `Pipeline`                        | New crate only     |
| Native values            | Extend `RuntimeValue` with a native variant                | Runtime crate only |

## 16. Examples

### 16.1 Self-describing document with interpolation

```yaml
fields:
  name:
    type: string
  version:
    type: number
  greeting:
    type: string
  doubled:
    type: number
functions:
  mul:
    params:
      x: { type: number }
      y: { type: number }
    return: { type: number }
  double:
    params:
      x: { type: number }
    return: { type: number }
    body: "mul(x, 2)"
name: alice
version: 1
greeting: "hello, ${doc.name}!"
doubled: "${double(version)}"
```

After processing:

- `name` → `"alice"` (string, valid)
- `version` → `1` (number, valid)
- `greeting` → `"hello, alice!"` (string, valid)
- `doubled` → `2` (number, valid — `double` is user-defined via `body: "mul(x, 2)"`)

### 16.2 Computed fields

Fields with an `expr:` key are computed after interpolation. The expression uses the same syntax as `${...}` content: function calls, path lookups, and literals. Computed fields can reference other fields (via the `doc` namespace) and can depend on other computed fields — multiple passes resolve dependencies in order.

```yaml
fields:
  version:
    type: number
  doubled:
    type: number
    expr: "mul(version, 2)"
  quadrupled:
    type: number
    expr: "mul(doubled, 2)"
functions:
  mul:
    params:
      x: { type: number }
      y: { type: number }
    return: { type: number }
version: 3
```

After processing:

- `version` → `3` (number, valid)
- `doubled` → `6` (computed: `mul(3, 2)`)
- `quadrupled` → `12` (computed: `mul(6, 2)` — references `doubled` from the previous pass)

If a computed field is also present in the data, the `expr:` value takes precedence. If the expression cannot be evaluated (e.g. references an unknown function), an error is reported.

### 16.3 Nested function calls

```yaml
fields:
  sum:
    type: number
functions:
  add:
    params:
      x: { type: number }
      y: { type: number }
    return: { type: number }
sum: "${add(add(1, 2), 3)}"
```

After processing: `sum` → `6`.

### 16.4 Environment variable interpolation

```yaml
fields:
  home_dir:
    type: string
home_dir: "${env.HOME}"
```

### 16.5 Shorthand path syntax

```yaml
fields:
  name:
    type: string
  greeting:
    type: string
name: alice
greeting: "hello, ${name}!"
```

`${name}` is equivalent to `${doc.name}`.

### 16.6 User-defined function composition

```yaml
fields:
  quadrupled:
    type: number
functions:
  mul:
    params:
      x: { type: number }
      y: { type: number }
    return: { type: number }
  double:
    params:
      x: { type: number }
    return: { type: number }
    body: "mul(x, 2)"
  quadruple:
    params:
      x: { type: number }
    return: { type: number }
    body: "double(double(x))"
quadrupled: "${quadruple(5)}"
```

After processing: `quadrupled` → `20` (`quadruple` calls `double` which calls `mul`).

### 16.7 Importing external documents

A field with `expr: "import('path')"` loads an external file and uses its data as the field value. The imported document's `fields:` and `functions:` are stripped from the data, but declared `functions:` are merged into the importer's function table — making imported functions available to the importer's expressions.

**`common.yml`:**

```yaml
fields:
  host:
    type: string
  port:
    type: number
host: example.com
port: 8080
```

**`main.yml`:**

```yaml
fields:
  common:
    type: object
    expr: "import('./common.yml')"
  host:
    type: string
    expr: "common.host"
  port:
    type: number
    expr: "common.port"
```

After processing:

- `common` → `{ host: "example.com", port: 8080 }` (loaded from `common.yml`, fields stripped)
- `host` → `"example.com"` (computed from imported data)
- `port` → `8080` (computed from imported data)

Paths resolve relative to the input file. Results are cached by resolved path — the same file is loaded once even if `import()` is called multiple times.

#### Importing functions

If the imported document declares `functions:`, those function signatures are merged into the importer's `FunctionResolver`. The importer's own functions take precedence; first import wins for duplicates. Imported function bodies are evaluated using the importer's resolver, so they can call other functions (including other imported ones).

**`lib.yml`:**

```yaml
fields:
  host:
    type: string
functions:
  make_label:
    params:
      h: { type: string }
    return: { type: string }
    body: 'concat(h, "-service")'
host: example.com
```

**`main.yml`:**

```yaml
fields:
  lib:
    type: object
    expr: "import('./lib.yml')"
  label:
    type: string
    expr: "make_label(lib.host)"
```

After processing: `label` → `"example.com-service"` (using the imported `make_label` function).

#### Dynamic import paths

The `import()` path argument can be computed at runtime using function calls. The expression engine evaluates nested calls before `import`, so dynamic paths work naturally:

```yaml
fields:
  common:
    type: object
    expr: "import(concat('./common_', 'base.yml'))"
```

This evaluates `concat('./common_', 'base.yml')` → `'./common_base.yml'`, then loads the file. Enables environment-driven or conditional loading when combined with `env` namespace references.

### 16.8 Scoped inheritance

```yaml
fields:
  permission_mode:
    type: string
    inheritable: true
  config:
    type: object
permission_mode: "read-only"
config:
  message: "${permission_mode}"
  server:
    permission_mode: "read-write"
    message: "${permission_mode}"
  deep:
    message: "${permission_mode}"
```

After processing:

- `config.message` → `"read-only"` (inherited from root)
- `config.server.message` → `"read-write"` (inherited from `config.server` override)
- `config.deep.message` → `"read-only"` (inherited from `config`, no override)

### 16.9 Merge fields (`merge: true`)

A field with `merge: true` spreads its computed value (must be an object) into the parent scope after computation, then removes the field itself. This closes the merge gap: instead of accessing imported data via a prefix (`${_base.name}`), the contents are flattened into the parent.

The pipeline gains a post-compute merge step: `extract → interpolate → compute → merge → validate`. Validation runs after merge on the final state — no new validation logic needed. Merge fields are excluded from schema validation since they've been spread and removed.

Conflict resolution: parent wins — explicit declarations take precedence over merged values. Multiple `merge: true` fields merge in declaration order; earlier merges fill keys that later merges cannot overwrite. Top-level spread only (no deep merge). Works with any computed field, not just `import()`.

**`base.yml`:**

```yaml
fields:
  host:
    type: string
  port:
    type: number
host: example.com
port: 8080
```

**`main.yml`:**

```yaml
fields:
  _base:
    type: object
    merge: true
    expr: "import('./base.yml')"
  host:
    type: string
  port:
    type: number
```

After processing:

- `_base` is removed after merge
- `host` → `"example.com"` (from merged `_base`)
- `port` → `8080` (from merged `_base`)

### 16.10 Parameterized computed fields (`arg:`)

A field's own data value becomes available as `${arg}` inside its `expr:` expression. Normally, `expr:` fields have their data value ignored or overridden. With `arg:`, the data value is an input to the computation.

The parameter is scoped to the field and disappears after computation. An optional `arg:` property declares the expected type (validated before the expr runs); without it, the value is available untyped. Backward-compatible — existing `expr:` fields that don't reference `${arg}` behave unchanged.

Path indexing applies naturally: `${arg}` for a scalar, `${arg.0}` for an array element, `${arg.host}` for an object key.

```yaml
fields:
  greeting:
    type: string
    expr: 'concat(arg, "!")'
    arg: true
functions:
  concat:
    params:
      x: { type: string }
      y: { type: string }
    return: { type: string }
greeting: hello
```

After processing: `greeting` → `"hello!"` (the field's data value `"hello"` is passed as `${arg}`).

With typed arg validation:

```yaml
fields:
  doubled:
    type: number
    expr: "mul(arg, 2)"
    arg: { type: number }
doubled: 5
```

If `doubled` were a string, the arg type check would fail before the expression runs.

## 17. Future directions

- **Full JSON Schema** — replace the minimal validator with a complete implementation (draft 2020-12).
- **Additional typegen backends** — TypeScript `.d.ts`, Rust structs, Python `TypedDict`.
- **Additional module resolvers** — `resolver-git`, `resolver-http` for loading documents from remote sources.
- **Integer subtyping** — `integer` as a subtype of `number` in the type system.
- **AOT optimization** — pre-evaluate static `import()` paths at compile time instead of runtime.

## 18. Emergent patterns

The AC language kernel is intentionally small — seven primitives:

| Primitive     | What it does                               |
| ------------- | ------------------------------------------ |
| `fields:`     | Declare schema for data                    |
| `functions:`  | Declare callable functions with `body:`    |
| `expr:`       | Compute a field's value                    |
| `arg:`        | Feed a field's data value into its `expr:` |
| `merge: true` | Spread computed object into parent scope   |
| `import()`    | Host function: load external document      |
| `${...}`      | Interpolate paths and function calls       |

The language's expressive power comes from **composition** of these primitives, not from adding keywords. Patterns that would require dedicated syntax in other languages emerge naturally from field declarations.

### The `import` field

The most striking example: the language describes its own module system using its own field syntax. Four primitives compose to produce an `import` field — no `import` keyword needed:

```yaml
fields:
  import:
    type: object
    arg: { type: string }
    expr: "import(arg)"
    merge: true
  host:
    type: string
  port:
    type: number

import: "./base.yml"
```

How it works:

- **`arg: { type: string }`** — the field's data value (`"./base.yml"`) is the input, type-checked
- **`expr: "import(arg)"`** — call `import()` on that path
- **`type: object`** — the output must be an object (required by merge)
- **`merge: true`** — spread the object into the parent scope, remove the `import` field

The field name `import` is just a regular name — not a reserved keyword. The language bootstraps its own import system from four primitives. This pattern is published as a reusable field definition in `libraries/lang/import.field.yml`.

### Other emergent patterns

The same principle produces other patterns without compiler support:

- **`config` field** — `arg: { type: object }` + `expr: "arg"` + `merge: true`: spread a data object into scope without importing a file
- **Conditional import** — `arg: { type: string }` + `expr: "import(concat(arg, env.ENV))"`: environment-driven loading
- **Chained import** — compute a path with one field, import it with another: `expr: "import(computed_path)"`

### The principle

The kernel stays small. The library of patterns grows. Users can invent their own field conventions without touching the compiler. The language is its own metalanguage — it describes its own extensions using its own field declaration syntax.
