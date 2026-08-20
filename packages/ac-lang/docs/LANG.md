# AC Language Specification

**Version:** 0.1.0
**Status:** Draft

## 1. Overview

AC is a configuration and expression language for self-describing documents. A document can declare its own structure (`fields:`), define functions (`functions:`), compute values (`expr:`), interpolate (`${...}`), and load external documents — all validated by a schema system.

The language is designed to be portable: the specification is independent of any particular runtime or host language. See [RUNTIME.md](./RUNTIME.md) for the reference runtime implementation and [DIALECT.md](./DIALECT.md) for the dialect interface that abstracts the canonical syntax in implementations.

### Design principles

1. **Declarative.** A document describes _what_ values it contains, not _how_ to compute them. Computation is expressed through field declarations, not imperative code.
2. **Schema-first.** Every field has a schema. Validation is not optional — it is part of the language semantics.
3. **Functions are host-provided.** The language defines no built-in functions. All capabilities (`add`, `mul`, `concat`, `len`, `import`, etc.) come from the runtime host.
4. **Composition over keywords.** The language has a small set of primitives. Complex patterns (imports, config spreading, conditional loading) emerge from composition, not from dedicated syntax.
5. **Format-agnostic.** Documents can be written in YAML, JSON, TOML, or any format that maps to JSON-compatible values.

## 2. Document structure

A document is a JSON-compatible value (typically an object) that may contain two reserved top-level keys:

- **`fields:`** — declares field schemas for the document's data fields.
- **`functions:`** — declares function signatures (param and return schemas, optional body).

Both keys are extracted and removed from the data before interpolation and validation.

**Reserved keys:** `fields` and `functions` cannot be declared as field names within `fields:`.

### 2.1 Field extraction

Extracting fields separates a document into:

- **Schemas** — field name to raw schema mapping, from the `fields:` section.
- **Functions** — function name to signature mapping, from the `functions:` section.
- **Data** — the document with `fields:` and `functions:` removed.
- **Metadata** — inheritable, immutable, computed, and merge field lists (extracted from schemas in a single pass).

Returns an error if a declared field name is reserved (`fields` or `functions`).

## 3. Value types

Values are JSON-compatible:

| Type    | Literal forms        |
| ------- | -------------------- |
| null    | `null`               |
| boolean | `true`, `false`      |
| number  | `42`, `1.5`, `-3`    |
| string  | `"hello"`, `'hello'` |
| array   | `[1, 2, 3]`          |
| object  | `{ "key": "value" }` |

Numbers are IEEE 754 doubles (JSON `number`). There is no integer subtype — `42` and `42.0` are the same type.

## 4. Expression syntax

Expressions are used inside `${...}` interpolation, `expr:` computed fields, and `body:` function definitions. The syntax is the same in all contexts.

### 4.1 Literals

| Literal | Syntax                 | Type    |
| ------- | ---------------------- | ------- |
| String  | `"hello"` or `'hello'` | string  |
| Number  | `42`, `1.5`, `-3`      | number  |
| Boolean | `true`, `false`        | boolean |
| Null    | `null`                 | null    |

### 4.2 Path lookups

A dotted path resolves a value from namespaces:

| Form         | Example            | Resolves to                   |
| ------------ | ------------------ | ----------------------------- |
| `ns.key`     | `doc.name`         | Namespace `doc`, field `name` |
| `ns.key.sub` | `doc.author.email` | Nested path                   |
| `key`        | `name`             | Shorthand for `doc.name`      |

**Shorthand:** If the first segment is not a known namespace, the resolver tries the default namespace. By convention the default is `doc`, so `name` is equivalent to `doc.name`. The default namespace name is a runtime host decision.

### 4.3 Function calls

| Form                | Example                 |
| ------------------- | ----------------------- |
| `fn()`              | `now()`                 |
| `fn(args)`          | `add(1, 2)`             |
| `fn(ns.key, "lit")` | `concat(doc.name, "!")` |
| `fn(fn(a, b), c)`   | `add(mul(2, 3), 4)`     |

**Discriminator:** `()` distinguishes function calls from path lookups. If the expression contains `(`, it's a function call. Otherwise, it's a path lookup.

Arguments are comma-separated and evaluated left-to-right. Each argument is itself an expression (literal, path, or nested call).

### 4.4 Namespaces

Namespaces are a language-level concept: a map of name to JSON value, used as the scope for path lookups. The language defines the namespace mechanism and a **default namespace** for shorthand resolution (bare `name` → `default.name`).

Which namespaces exist and what the default is called are **runtime host decisions**, not language-level features. The reference runtime conventionally provides:

| Namespace | Source                        | Example         |
| --------- | ----------------------------- | --------------- |
| `doc`     | Document's own data fields    | `${doc.name}`   |
| `env`     | Process environment variables | `${env.HOME}`   |
| custom    | Added by the runtime host     | `${custom.key}` |

A runtime host may provide any namespaces it chooses (e.g., secrets, metadata, user input). See [RUNTIME.md](./RUNTIME.md) for the reference runtime's namespace configuration.

## 5. Interpolation

Interpolation recursively replaces `${...}` patterns in string values. Recursion covers objects, arrays, and nested strings.

### 5.1 Syntax

| Syntax                 | Kind                          | Example                      |
| ---------------------- | ----------------------------- | ---------------------------- |
| `${ns.key}`            | Namespace path                | `${doc.name}`, `${env.HOME}` |
| `${ns.key.sub}`        | Nested path                   | `${doc.author.email}`        |
| `${key}`               | Shorthand path                | `${name}` → `doc.name`       |
| `${fn()}`              | Function call (no args)       | `${now()}`                   |
| `${fn(args)}`          | Function call                 | `${add(1, 2)}`               |
| `${fn(ns.key, "lit")}` | Function call with mixed args | `${concat(doc.name, "!")}`   |
| `${fn(fn(a, b), c)}`   | Nested function calls         | `${add(mul(2, 3), 4)}`       |

### 5.2 Type preservation

When the entire string value is a single `${...}` expression, the raw value is returned (preserving type). When the string contains other text, all `${...}` patterns are stringified and concatenated.

| Value                          | Result            | Type   |
| ------------------------------ | ----------------- | ------ |
| `"${mul(version, 2)}"`         | `2`               | number |
| `"result: ${mul(version, 2)}"` | `"result: 2"`     | string |
| `"hello, ${doc.name}!"`        | `"hello, alice!"` | string |

### 5.3 Argument forms

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

## 6. Field declarations

Fields are declared in the `fields:` section. Each field has a schema (type and constraints) and optional AC-specific keys (metadata).

### 6.1 Schema

Schemas use [JSON Schema draft 2020-12](https://json-schema.org/) vocabulary. The reference runtime validates fields with a full draft 2020-12 validator — all standard keywords are supported.

The schema vocabulary is defined by the [dialect](./DIALECT.md) — the canonical dialect uses JSON Schema draft 2020-12. The key names used in field declarations (`fields`, `functions`, `expr`, `arg`, `merge`, `inheritable`, `mutable`) are also defined by the dialect.

The `type` keyword declares the field's type. Supported types: `string`, `number`, `integer`, `boolean`, `null`, `array`, `object`. Multi-type is allowed via arrays (`["string", "null"]`).

```yaml
fields:
  name:
    type: string
  version:
    type: number
  tags:
    type: array
    items:
      type: string
  config:
    type: object
    properties:
      host:
        type: string
      port:
        type: number
```

**Validation keywords** (non-exhaustive — see the JSON Schema spec for the full list):

- **String**: `minLength`, `maxLength`, `pattern`
- **Number**: `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`
- **Array**: `items`, `prefixItems`, `minItems`, `maxItems`, `uniqueItems`, `contains`
- **Object**: `required`, `properties`, `patternProperties`, `additionalProperties`, `minProperties`, `maxProperties`
- **Combinators**: `allOf`, `anyOf`, `oneOf`, `not`
- **Conditional**: `if` / `then` / `else`
- **Enum/const**: `enum`, `const`
- **References**: `$ref`, `$defs`
- **Boolean schemas**: `true` (accepts anything), `false` (rejects everything)

Example with constraints:

```yaml
fields:
  mode:
    type: string
    enum: [read-only, read-write, admin]
  port:
    type: integer
    minimum: 1
    maximum: 65535
  tags:
    type: array
    items:
      type: string
    minItems: 1
    uniqueItems: true
  config:
    type: object
    properties:
      host:
        type: string
        pattern: "^[a-z]"
      retries:
        type: integer
        minimum: 0
    required: [host]
    additionalProperties: false
```

**Reserved field names**: `fields` and `functions` cannot be declared as field names — they are reserved top-level keys.

### 6.2 Computed fields (`expr:`)

Fields with an `expr:` key are computed after interpolation. The expression uses the same syntax as `${...}` content (without the `${}` envelope):

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
```

Computation runs in passes. Before each pass, the default namespace (conventionally `doc`) is populated with all known field values (data values plus results from previous passes). During a pass, each computed field's `expr:` is evaluated **once** — the result is stored in the namespace, and any other expression referencing that field reads the stored value rather than triggering a re-evaluation. Multiple passes resolve dependencies between computed fields — if field A depends on field B, B is computed in an earlier pass and A reads its stored value in the next pass. If no progress is made in a pass (circular or unresolvable dependencies), errors are reported for the remaining fields.

If a computed field is also present in the data, the `expr:` value takes precedence — the expression runs and its result replaces the data value:

```yaml
fields:
  greeting:
    type: string
    expr: 'concat(arg, "!")'
greeting: hello
```

Here `greeting` has a data value (`"hello"`) and an `expr:`. The expression runs with `arg` bound to `"hello"`, producing `"hello!"`, which replaces the original data value. Using `${arg}` is optional — an `expr:` field that doesn't reference `${arg}` behaves the same as before (it simply doesn't see its own data value).

### 6.3 Parameterized computed fields (`arg:`)

When a field has both a data value and an `expr:`, the data value is automatically available as `${arg}` inside the expression. No declaration is needed — `arg` is always bound when data is present.

The parameter is scoped to the field and disappears after computation. The optional `arg:` property is only for **type validation** — it declares the expected type and is checked before the expression runs. Without `arg:`, the value is available untyped.

Path indexing applies naturally: `${arg}` for a scalar, `${arg.0}` for an array element, `${arg.host}` for an object key.

```yaml
fields:
  greeting:
    type: string
    expr: 'concat(arg, "!")'
greeting: hello
```

After processing: `greeting` → `"hello!"`.

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

Fields with `expr:` that don't reference `${arg}` are unaffected — the feature is purely opt-in by using `${arg}` in the expression.

### 6.4 Merge fields (`merge: true`)

A field with `merge: true` spreads its computed value (must be an object) into the parent scope after computation, then removes the field itself. This flattens imported or computed data into the parent.

Conflict resolution: parent wins — explicit declarations take precedence over merged values. Multiple `merge: true` fields merge in declaration order; earlier merges fill keys that later merges cannot overwrite. Top-level spread only (no deep merge).

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

After processing: `_base` is removed; `host` and `port` are spread from the imported data.

### 6.5 Scoped inheritance (`inheritable: true`)

Fields declared `inheritable: true` propagate their values into nested objects. As scoped interpolation descends into objects, inheritable fields are merged into the default namespace (conventionally `doc`). Child objects inherit the parent's scope and can override inheritable fields locally — the override applies to that subtree only.

If no fields are inheritable, scoped interpolation delegates to standard interpolation — no scope tracking overhead.

**How it works:**

1. At each object level, collect local inheritable field values.
2. Merge with parent scope (local overrides win).
3. Inject merged scope into the default namespace for children.
4. Recurse with updated scope and namespace.

```yaml
fields:
  permission_mode:
    type: string
    inheritable: true
  config:
    type: object
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

### 6.6 Immutability (`mutable: false`)

Fields declared `mutable: false` cannot be overridden in nested scopes.

Validation walks the data tree. For each immutable field, it compares every nested occurrence against the root value. Returns error messages for any mismatches.

## 7. Function declarations

Functions are declared in the `functions:` section. A function signature declares:

- **Params** — parameter name to schema mapping.
- **Return schema** — return type schema, if declared.
- **Body** — expression body string, if user-defined.

```yaml
functions:
  add:
    params:
      x: { type: number }
      y: { type: number }
    return: { type: number }
```

Functions with a `body:` field are **user-defined** — evaluated by parsing the body as an expression. Functions without a `body:` field are **host primitives** — resolved by the runtime's function resolver.

```yaml
functions:
  double:
    params:
      x: { type: number }
    return: { type: number }
    body: "mul(x, 2)"
```

### 7.1 User-defined function bodies

A `body:` expression is evaluated as follows:

1. Params are merged into the default namespace (conventionally `doc`, shadowing fields with the same name).
2. The body expression is evaluated as an expression string.
3. Bare names like `x` resolve via shorthand path lookup (→ `doc.x` by convention).
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

### 7.2 Typed parameter validation

Function arguments and return values are validated using the schema system — the same system used for field validation. This means function params and return values are validated by the same schema language as fields.

## 8. Module system

The language defines no built-in functions. Module loading is a **runtime host capability** — the reference runtime conventionally provides an `import()` host function that loads an external document by path. The imported document's `fields:` and `functions:` are stripped from the data, but declared `functions:` are merged into the importer's function table — making imported functions available to the importer's expressions.

A runtime host may provide module loading under a different function name, through a different mechanism, or not at all. The module system semantics described below apply to the reference runtime's `import()` convention.

### 8.1 Import

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

Paths resolve relative to the input file. Results are cached by resolved path — the same file is loaded once even if `import()` is called multiple times.

### 8.2 Importing functions

If the imported document declares `functions:`, those function signatures are merged into the importer's function resolver. The importer's own functions take precedence; first import wins for duplicates. Imported function bodies are evaluated using the importer's resolver, so they can call other functions (including other imported ones).

```yaml
# lib.yml
functions:
  make_label:
    params:
      h: { type: string }
    return: { type: string }
    body: 'concat(h, "-service")'
host: example.com

# main.yml
fields:
  lib:
    type: object
    expr: "import('./lib.yml')"
  label:
    type: string
    expr: "make_label(lib.host)"
```

After processing: `label` → `"example.com-service"`.

### 8.3 Dynamic import paths

The `import()` path argument can be computed at runtime using function calls:

```yaml
fields:
  common:
    type: object
    expr: "import(concat('./common_', 'base.yml'))"
```

This evaluates `concat('./common_', 'base.yml')` → `'./common_base.yml'`, then loads the file. Enables environment-driven or conditional loading when combined with runtime-provided namespaces (e.g., `env` in the reference runtime).

### 8.4 Import sources

The runtime determines where `import()` paths resolve. The reference runtime supports:

- **File paths** — relative to the importing document (`./base.yml`, `../common/config.yml`).
- **HTTP URLs** — `https://example.com/schemas/base.yml`.
- **Git URLs** — `git+https://github.com/org/repo.git@main/schemas/base.yml`.

See [RUNTIME.md](./RUNTIME.md) for resolver configuration.

## 9. Type system

The type system provides static type checking of AC documents before runtime evaluation. It is schema-system-agnostic — a schema interpreter converts any schema language's vocabulary into a neutral type enum.

### 9.1 Type representation

| Variant    | Meaning                                                      |
| ---------- | ------------------------------------------------------------ |
| `String`   | String type                                                  |
| `Number`   | Number type (includes integers — no integer subtyping yet)   |
| `Boolean`  | Boolean type                                                 |
| `Null`     | Null type                                                    |
| `Array(T)` | Array with element type `T`                                  |
| `Object`   | Object with named property types                             |
| `Function` | Function type — parameter names/types and return type        |
| `Unknown`  | Cannot be statically determined — compatible with everything |
| `Never`    | Impossible code path — compatible with everything            |

`Unknown` represents types that cannot be statically determined (e.g., an unresolvable path or unknown function). `Never` represents uninhabited code paths. Both are compatible with all other types.

### 9.2 Type environment

The type environment maps field names to their declared types. It supports scope chaining for inheritance: a child scope inherits all bindings from its parent and can override individual fields. Lookups walk the chain from child to parent.

### 9.3 Schema interpreter

A schema interpreter converts a raw schema fragment into a type. The default interpreter maps JSON Schema `type` keywords to types: `string`, `number`/`integer`, `boolean`, `null`, `array` (with `items`), `object` (with `properties`), and `enum` (infers type from first enum value).

### 9.4 Expression type inference

Expression strings (same syntax as `${...}` and `expr:` fields) can be statically type-inferred given a type environment and function table.

**Expression AST:**

| Variant   | Form               | Example                                    |
| --------- | ------------------ | ------------------------------------------ |
| `Literal` | Typed literal      | `"hello"`, `42`, `true`, `null`            |
| `Path`    | Dotted path lookup | `doc.name`, `env.HOME`, `name` (shorthand) |
| `Call`    | Function call      | `add(1, 2)`, `mul(x, 2)`                   |

Inference walks the AST:

- **Literal** → the literal's type.
- **Path** → looks up the first segment in the type environment; if not found, tries the default namespace shorthand. Walks remaining segments into object types.
- **Call** → looks up the function in the function table, infers argument types, checks compatibility with param types, and returns the function's return type. Returns `Unknown` if the function is not found or argument counts mismatch.

### 9.5 Type compatibility

`is_compatible(actual, expected)` checks whether `actual` can be used where `expected` is required:

- `Unknown` is compatible with anything (both directions).
- `Never` is compatible with anything (both directions).
- Exact primitive matches are compatible.
- `Array<T>` is compatible if item types are compatible.
- `Object` is compatible if every expected property exists with a compatible type. Extra properties in `actual` are allowed.
- `Function` is compatible if param count matches and all param/return types are compatible.

### 9.6 Per-key type checking

Each AC-specific key has its own type check:

| Key           | Check                                                              |
| ------------- | ------------------------------------------------------------------ |
| `expr`        | Inferred expression type must be compatible with declared type     |
| `arg`         | Arg type must be compatible with field's declared type             |
| `merge`       | Merge fields must be `object`; merged props compatible with parent |
| `inheritable` | Override values in nested data must be type-compatible with root   |
| `mutable`     | Immutable fields (`mutable: false`) must not be overridden         |
| `import()`    | Imported field types must be compatible with parent scope          |

### 9.7 Type errors

A type error carries:

- **field** — the field name where the error was detected.
- **expected** — the expected type, if known.
- **actual** — the actual (inferred) type, if known.
- **message** — human-readable explanation.
- **path** — dotted path to the location within the document (e.g., `config.server`).
- **location** — optional source location (line:column), populated by the runtime after type checking.

## 10. Type generation

Type generation produces clean schema output from field declarations. It strips AC-specific keys (`expr`, `arg`, `merge`, `inheritable`, `mutable`), resolves imports, and emits a schema document via a pluggable backend.

### 10.1 Processing steps

1. **Filter merge fields** — merge fields are excluded from output (they're spread and removed at runtime).
2. **Strip AC keys** — `expr`, `arg`, `merge`, `inheritable`, `mutable` are removed from all field schemas.
3. **Merge imports** — imported module schemas are merged in (parent wins — existing fields are not overwritten).

### 10.2 Backends

| Backend     | Output      | Extension      |
| ----------- | ----------- | -------------- |
| JSON Schema | JSON Schema | `.schema.json` |

The backend also provides an IDE hint comment (e.g., `# yaml-language-server: $schema=<path>`) for editor integration and a file extension for emitted files.

Future backends: TypeScript `.d.ts`, Python `TypedDict`.

## 11. Processing pipeline

### 11.1 Runtime pipeline

```
Input file (YAML/JSON/TOML)
     │
     ▼
  Parse → JSON-compatible value
     │
     ▼
  Extract fields → schemas, functions, data
     │
     ▼
  Build namespaces { doc: data, env: env_vars, ... }  (runtime host decides which)
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

### 11.2 Compile-time pipeline

```
Input file (YAML/JSON/TOML)
     │
     ▼
  Parse → JSON-compatible value
     │
     ▼
  Extract → schemas, functions, data, metadata
     │
     ├── Typegen path:
     │      ▼
     │   Resolve imports → build typegen context
     │      │  ├── Filter merge fields
     │      │  ├── Strip AC keys (expr, arg, merge, inheritable, mutable)
     │      │  └── Resolve imports
     │      ▼
     │   Generate → output schema (JSON Schema)
     │
     └── Typecheck path:
            ▼
         Typecheck → type errors
            │  ├── Check expr key types (inferred vs declared)
            │  ├── Check arg key types (arg vs declared)
            │  ├── Check merge key types (must be object, props compatible)
            │  ├── Check inheritable key types (overrides compatible)
            │  ├── Check mutable key (immutable not overridden)
            │  └── Check import types (cross-module compatibility)
            ▼
         Output: ok / N type error(s)
```

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

`${name}` is equivalent to `${doc.name}` (by convention — the default namespace is a runtime host decision).

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

### 12.9 Merge fields

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

### 12.10 Parameterized computed fields

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

## 13. Emergent patterns

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

The field name `import` is just a regular name — not a reserved keyword. The language bootstraps its own import system from four primitives.

### Other emergent patterns

- **`config` field** — `arg: { type: object }` + `expr: "arg"` + `merge: true`: spread a data object into scope without importing a file
- **Conditional import** — `arg: { type: string }` + `expr: "import(concat(arg, env.ENV))"`: environment-driven loading
- **Chained import** — compute a path with one field, import it with another: `expr: "import(computed_path)"`

### The principle

The kernel stays small. The library of patterns grows. Users can invent their own field conventions without touching the compiler. The language is its own metalanguage — it describes its own extensions using its own field declaration syntax.

## 14. Future directions

- **Additional typegen backends** — TypeScript `.d.ts`, Rust structs, Python `TypedDict`.
- **Integer subtyping** — `integer` as a subtype of `number` in the type system.
- **AOT optimization** — pre-evaluate static `import()` paths at compile time instead of runtime.
- **Source location tracking for runtime errors** — extend source map usage to interpolation and computation errors, not just type errors.
