# AC Language Technical Specification

**Version:** 0.1.0
**Status:** Draft

## 1. Overview

AC is a configuration and expression language built around three orthogonal axes:

- **Values** — JSON-compatible data represented by `RuntimeValue`.
- **Schemas** — abstract validation via a generic schema interface; JSON Schema is the default implementation.
- **Execution** — expression trees evaluated by a VM with host-provided functions.

A document can describe its own structure (`fields:`), declare function signatures (`functions:`), and interpolate values (`${...}`) — all validated by the same schema system.

### Design principles

1. **Core is dependency-free.** The core module defines expression trees and field bindings with zero external dependencies.
2. **Frontends are pluggable.** Any format that parses into `RuntimeValue` and serializes back can implement the frontend interface.
3. **Schemas are pluggable.** Any validation system can implement the schema interface. JSON Schema is one implementation, not the abstraction.
4. **Functions are host-provided.** The VM has no built-in functions. All capabilities come from the host.
5. **Interpolation is domain logic.** Field extraction, interpolation, and function call resolution live in the fields module, not CLI plumbing.

### Naming: "fields" not "keys"

The reserved `fields:` key declares typed, validated members of a document. The term "fields" is deliberate:

- **Fields imply schema.** A field has a name, a type, and constraints. "Keys" is generic — every object property in JSON/YAML is already a key. Overloading `keys:` to mean "schema declarations" makes a common term ambiguous.
- **Self-reference breaks.** Today `fields:` is a reserved key that cannot be declared as a field name. Renaming to `keys:` yields "keys is a reserved key that cannot be declared as a key name" — the sentence collapses.
- **Convention.** Schema languages (protobuf, Avro, SQL, JSON Schema's "properties") use "fields" or "properties" for typed, declared members. "Keys" is the substrate — the raw object entries in a key-value store. ac-lang is in the schema camp.
- **`Field<V>` is foundational.** The core type `Field<V>` is a named binding of a value. "Key" is an identifier; "field" is a name + value + schema.

## 2. Architecture

### Module responsibilities

| Module            | Responsibility                                                                         | Dependencies          |
| ----------------- | -------------------------------------------------------------------------------------- | --------------------- |
| **core**          | Expression trees and field bindings — generic, no external dependencies                | None                  |
| **runtime**       | Concrete value type and frontend interface                                             | core                  |
| **vm**            | Execution engine, host interface, execution context, errors                            | core, runtime         |
| **schema**        | Abstract schema interface and validation error                                         | None                  |
| **schema-json**   | JSON Schema implementation of the schema interface                                     | schema, runtime       |
| **fields**        | Field extraction, interpolation, computed fields, function call resolution, validation | core, runtime, schema |
| **frontend-json** | JSON frontend                                                                          | runtime               |
| **frontend-yaml** | YAML frontend                                                                          | runtime               |
| **frontend-toml** | TOML frontend                                                                          | runtime               |
| **cli**           | `ac` CLI binary — parse, evaluate, validate, fields                                    | all of the above      |

### Dependency constraints

- **core** and **schema** have zero external dependencies.
- **runtime** is the only module that couples to a JSON value library.
- **fields** does not depend on **vm** — the caller bridges the two via a function resolver interface.
- All other modules depend on one or both of the core modules.

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

| Module          | Format |
| --------------- | ------ |
| `frontend-json` | JSON   |
| `frontend-yaml` | YAML   |
| `frontend-toml` | TOML   |

Adding a new format (XML, INI, etc.) requires only implementing the frontend interface. The core, VM, and schema system are unaffected.

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

## 9. CLI

### 9.1 Commands

```
ac parse    --input <file> --format <yaml|json|toml>
ac eval     --input <file> --format <yaml|json|toml> --output <yaml|json|toml>
ac validate --input <file> --format <yaml|json|toml> --schema <schema.json>
ac fields   --input <file> --format <yaml|json|toml>
```

If `--input` is omitted, reads from stdin.

### 9.2 `parse`

Parses input in the specified format, outputs as YAML.

### 9.3 `eval`

Parses input, evaluates as an expression tree, outputs the result in the specified output format. If the input is an object, each key is defined as a variable in the execution context.

### 9.4 `validate`

Parses input, validates against an external JSON Schema file. Outputs `valid` or `invalid: <error>`.

### 9.5 `fields`

Processes a self-describing document:

1. **Extract** `fields:` and `functions:` sections.
2. **Build** namespaces (`doc` from data, `env` from environment).
3. **Interpolate** `${...}` patterns in all data values (path lookups + function calls).
4. **Compute** fields with `expr:` key (after interpolation, multiple passes for dependencies).
5. **Validate** each field against its declared schema.
6. **Report** undeclared fields.

### 9.6 Built-in host functions

The CLI provides these functions via its default host:

| Function | Args                        | Returns   | Description                                                                                                                                                                                                                      |
| -------- | --------------------------- | --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `add`    | `number...`                 | `number`  | Sum of all numeric args                                                                                                                                                                                                          |
| `mul`    | `number...`                 | `number`  | Product of all numeric args                                                                                                                                                                                                      |
| `concat` | `string...`                 | `string`  | Concatenation of all string args                                                                                                                                                                                                 |
| `len`    | `string \| array \| object` | `integer` | Length of the argument                                                                                                                                                                                                           |
| `import` | `string`                    | `object`  | Loads an external document by path, extracts its data (strips `fields:`/`functions:`), and returns it. Paths resolve relative to the input file. Results are cached by resolved path. Format is auto-detected by file extension. |

## 10. Processing pipeline

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

## 11. Extension points

| Extension                | How                                                  | Touches             |
| ------------------------ | ---------------------------------------------------- | ------------------- |
| New format               | Implement the frontend interface                     | New module only     |
| New schema language      | Implement the schema interface                       | New module only     |
| New host function        | Implement the function interface, register with host | Host impl only      |
| Custom namespace         | Insert into the namespaces map                       | Caller code         |
| Custom function resolver | Implement the function resolver interface            | Caller code         |
| Native values            | Extend `RuntimeValue` with a native variant          | Runtime module only |

## 12. Examples

### 12.1 Self-describing document with interpolation

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

### 12.2 Computed fields

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

### 12.3 Nested function calls

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

### 12.4 Environment variable interpolation

```yaml
fields:
  home_dir:
    type: string
home_dir: "${env.HOME}"
```

### 12.5 Shorthand path syntax

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

### 12.6 User-defined function composition

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

### 12.7 Importing external documents

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

### 12.8 Scoped inheritance

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

### 12.9 Merge fields (`merge: true`)

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

### 12.10 Parameterized computed fields (`arg:`)

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

## 13. Future directions

- **Full JSON Schema** — replace the minimal validator with a complete implementation (draft 2020-12).
- **Module system** — `import()` host function for loading external documents as computed fields. A field with `expr: "import('./common.yml')"` loads the file via the host's `FunctionResolver` and injects the result as the field's value. The host controls I/O (filesystem, HTTP, embedded), caching, and AOT optimization for static path arguments. No new traits or pipeline stages — `import()` is a regular host function, consistent with the existing `add`/`mul`/`concat`/`len` pattern. Scoped imports work via nested `fields:` declarations.

## 14. Emergent patterns

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
