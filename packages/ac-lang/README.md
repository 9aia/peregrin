# ac-lang

A small language runtime where **schemas define types** and **frontends parse formats**. The core is agnostic of both.

## Core idea

The kernel does not define its own type system or format. There is no Rust enum re-implementing JSON's types, no hardcoded YAML parser. Instead:

- **`RuntimeValue`** — a transparent newtype around `serde_json::Value`. Deliberately boring.
- **`Schema<V>`** — abstract validation trait. Any schema language can implement it.
- **Frontends** — parse any external format (JSON, YAML, TOML) into `RuntimeValue`.
- **VM** — evaluates expressions that produce `RuntimeValue`s. Knows nothing about types or formats.

```
Any frontend (JSON/YAML/TOML/...)
      │
      │ parses into
      ▼
RuntimeValue
      │
      │ validated by
      ▼
Any schema (JSON Schema/Cue/Custom/...)
      │
      ▼
"this is a User" · "this is a String" · ...
```

The VM genuinely doesn't know what a String, Number, Boolean, Object, Array, DateTime, or User is. Those semantics live in schemas. The core doesn't know what JSON, YAML, or TOML is. Those live in frontends.

## Crate structure

```
crates/
├── core/           # Expr<V>, Field<V> — generic, zero dependencies
├── runtime/        # RuntimeValue(serde_json::Value) + Frontend trait — the concrete value type
├── vm/             # Vm, Host, Function, Context
├── schema/         # Schema<V> trait + ValidationError — zero dependencies
├── schema-json/    # JsonSchema — JSON Schema implementation of Schema<RuntimeValue>
├── fields/         # Field extraction, interpolation, undeclared-field detection
├── frontend-json/  # JSON parse/serialize → RuntimeValue (implements Frontend)
├── frontend-yaml/  # YAML parse/serialize → RuntimeValue (implements Frontend)
├── frontend-toml/  # TOML parse/serialize → RuntimeValue (implements Frontend)
└── cli/            # `ac` CLI — parse, evaluate, validate (clap)
```

**`ac-core`** — the language model. `Expr<V>` (value or call) and `Field<V>` (name + value). Generic over the value type, zero dependencies. No knowledge of formats or schemas.

**`ac-runtime`** — the concrete value representation. `RuntimeValue` wraps `serde_json::Value` (private inner field, accessed via `inner()`/`inner_mut()`/`into_inner()`). Also defines the `Frontend` trait — the abstract interface for format parsers. This is the only crate that couples to `serde_json` for values. Core stays clean.

**`ac-vm`** — the execution engine. `Vm<H>` walks expression trees. `Host` resolves function names. `Function` is the trait host-provided callables implement. `Context` manages variable scopes. The VM does not depend on any schema or frontend crate.

**`ac-schema`** — abstract `Schema<V>` trait and `ValidationError`. Zero dependencies. Knows nothing about JSON Schema, `$ref`, `allOf`, `format`, or any concrete schema language. Any schema system can implement this trait.

**`ac-schema-json`** — JSON Schema implementation. `JsonSchema` implements `Schema<RuntimeValue>` with a built-in minimal validator (`type` and `required` keywords). Can be replaced with a full JSON Schema validator — or a different schema language entirely — behind the same trait.

**`ac-frontend-*`** — format frontends. Each implements the `Frontend` trait from `ac-runtime` and parses an external format into `RuntimeValue` / serializes back. JSON is trivial (already `serde_json::Value`). YAML and TOML convert through `serde_json::Value`. New frontends (XML, INI, etc.) can be added by implementing the `Frontend` trait — the core and VM are unaffected.

**`ac-fields`** — field extraction, interpolation, computed fields, and undeclared-field detection for self-describing documents. Provides `extract_fields()`, `interpolate()`, `compute_fields()`, `undeclared_fields()`, and namespace builders. Schema-agnostic — callers choose their own `Schema<V>` implementation.

**`ac-cli`** — command-line interface (`ac` binary). Parses YAML/JSON/TOML input via the `Frontend` trait, evaluates expressions, validates against schemas. Delegates field extraction and interpolation to `ac-fields`. Built with clap.

```
ac-core (no deps)          ac-schema (no deps)
    ↑                            ↑
ac-runtime (serde_json)    ac-schema-json (serde_json)
    ↑                            ↑
ac-vm                      ac-frontends    ac-fields
    ↑                            ↑                ↑
    └──────── ac-cli ────────────────────────────┘
```

The `Frontend` trait lives in `ac-runtime` and is implemented by each frontend crate. The CLI dispatches through `Box<dyn Frontend>` — adding a new format requires only implementing the trait, not modifying the CLI.

Core and schema have zero dependencies. Frontends and schema implementations are pluggable — swap JSON Schema for another validator, or YAML for another format, without touching the core or VM.

## Usage

### Library

```rust
use ac_core::Expr;
use ac_runtime::RuntimeValue;
use ac_vm::{Context, Error, Function, Host, Vm};

struct Now;

impl Function for Now {
    fn call(&self, _args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        Ok(RuntimeValue::new(serde_json::json!("2026-08-20T06:00:00Z")))
    }
}

struct MyHost { now: Now }

impl Host for MyHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "now" => Some(&self.now),
            _ => None,
        }
    }
}

let mut vm = Vm::new(MyHost { now: Now });
let mut ctx = Context::new();

let expr = Expr::call("now", vec![]);
let result = vm.eval(&expr, &mut ctx).unwrap();
// result is RuntimeValue("2026-08-20T06:00:00Z")
```

### CLI

```sh
# Parse a YAML file and output as YAML
ac parse --input config.yaml

# Parse JSON, evaluate, output as YAML
ac eval --input data.json --format json --output yaml

# Parse TOML, evaluate, output as JSON
ac eval --input config.toml --format toml --output json

# Validate a YAML file against a JSON Schema
ac validate --input config.yaml --schema user.schema.json

# Validate a TOML file against a JSON Schema
ac validate --input config.toml --format toml --schema user.schema.json

# Validate a self-describing file with inline fields:
ac fields --input config.yaml
```

### Self-describing documents

A file can declare its own field schemas inline using a `fields:` section. Each field is validated against its declared schema. The `fields:` key is a reserved built-in — it cannot be declared as a data field.

```yaml
fields:
  name:
    type: string
    enum:
      - alice
      - bob
      - charlie
  version:
    type: number
  tags:
    type: array
name: alice
version: 1
tags:
  - admin
  - user
```

The `fields:` key is extracted and each remaining key is validated against its corresponding schema. Undeclared fields are reported as errors.

The schema language used inside `fields:` is whatever the CLI is configured with — by default JSON Schema, but any `Schema<V>` implementation works.

### Variable interpolation

Field values can reference other fields and environment variables using `${...}` syntax. The interpolation system supports:

- **Namespace paths**: `${doc.field_name}`, `${env.VAR_NAME}`, `${doc.author.email}`
- **Shorthand paths**: `${name}` (equivalent to `${doc.name}`)
- **Function calls**: `${add(1, 2)}`, `${mul(version, 2)}`, `${concat(doc.name, "!")}`
- **Nested calls**: `${add(mul(2, 3), 4)}`

The discriminator is `()` — if the expression contains `(`, it's a function call. Otherwise, it's a path lookup.

```yaml
fields:
  name:
    type: string
  version:
    type: number
  greeting:
    type: string
  full_name:
    type: string
  home_dir:
    type: string
  doubled:
    type: number
  sum:
    type: number
functions:
  add:
    params:
      x: { type: number }
      y: { type: number }
    return: { type: number }
  mul:
    params:
      x: { type: number }
      y: { type: number }
    return: { type: number }
name: alice
version: 1
greeting: "hello, ${doc.name}!"
full_name: "${doc.name} v${doc.version}"
home_dir: "${env.HOME}"
doubled: "${mul(version, 2)}"
sum: "${add(add(1, 2), 3)}"
```

After interpolation:

- `greeting` → `"hello, alice!"`
- `full_name` → `"alice v1"`
- `home_dir` → `"/home/user"` (from `$HOME`)
- `doubled` → `2` (number, not string — type is preserved for single-expression values)
- `sum` → `6` (nested function calls)

The `doc` namespace is built from the document's own data fields. The `env` namespace is built from process environment variables. Custom namespaces can be added by the host — the interpolation engine just looks up the first segment as a namespace name and walks the rest as a dotted path.

### Function calls and typed parameters

Functions are declared in a `functions:` section with param and return schemas. Before a function is called, its arguments are validated against the declared param schemas. After the call, the return value is validated against the return schema. This uses the same `Schema<V>` system as field validation.

```yaml
functions:
  add:
    params:
      x: { type: number }
      y: { type: number }
    return: { type: number }
```

The `functions:` key is reserved, like `fields:`. Function implementations come from two sources:

1. **Host primitives** — provided by the host (the CLI provides `add`, `mul`, `concat`, `len`). These are Rust functions implementing the `Function` trait.
2. **User-defined functions** — declared inline with a `body:` field. No Rust code needed.

#### User-defined functions

A function with a `body:` field is evaluated by parsing the body as an expression and resolving it. Params are available as bare names. The body can call host primitives and other user-defined functions.

```yaml
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
```

- `double(5)` → evaluates `mul(x, 2)` with `x = 5` → `10`
- `quadruple(5)` → evaluates `double(double(x))` → `double(10)` → `20`

User-defined functions compose with each other and with host primitives. The body expression uses the same syntax as `${...}` content: function calls, path lookups, and literals. A recursion depth limit (64) prevents infinite self-reference.

Function calls in `${...}` preserve type: `${double(version)}` returns a number, not a stringified number. This only applies when the entire value is a single `${...}` expression — mixed strings like `"result: ${double(version)}"` are stringified.

The `FunctionResolver` trait in `ac-fields` bridges interpolation to the VM. The CLI implements it with `VmResolver`, which validates args, dispatches to the VM (for host primitives) or evaluates the body (for user-defined functions), and validates the return value. Custom hosts can provide their own resolver.

### Scoped inheritance

Fields can be declared `inheritable: true` to propagate their values into nested objects. Child objects inherit the parent's scope and can override inheritable fields locally — the override applies to that subtree only.

```yaml
fields:
  permission_mode:
    type: string
    inheritable: true
  config:
    type: object
  server:
    type: object
permission_mode: "read-only"
config:
  message: "${permission_mode}" # → "read-only" (inherited from root)
  server:
    permission_mode: "read-write" # override for this subtree
    message: "${permission_mode}" # → "read-write" (inherited from server)
  deep:
    message: "${permission_mode}" # → "read-only" (inherited from config, no override)
server:
  message: "${permission_mode}" # → "read-only" (inherited from root)
```

The scope is merged into the `doc` namespace as interpolation descends. Shorthand `${permission_mode}` resolves from the current scope — no new syntax needed.

Fields can also be declared `mutable: false` to prevent overrides in nested scopes:

```yaml
fields:
  env_name:
    type: string
    inheritable: true
    mutable: false
env_name: "prod"
config:
  env_name: "dev" # INVALID — mutable: false, cannot override
```

The `validate_mutable` function walks the data tree and reports any nested overrides of immutable fields.

### Computed fields

Fields with an `expr:` key are computed after interpolation. The expression uses the same syntax as `${...}` content: function calls, path lookups, and literals. Computed fields can reference other fields and can depend on other computed fields — multiple passes resolve dependencies in order.

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

- `version` → `3`
- `doubled` → `6` (computed: `mul(3, 2)`)
- `quadrupled` → `12` (computed: `mul(6, 2)` — references `doubled` from the previous pass)

If a computed field is also present in the data, the `expr:` value takes precedence. If the expression cannot be evaluated, an error is reported.

### Frontend-agnostic

The same document works regardless of frontend. This is valid JSON:

```json
{
  "fields": {
    "name": { "type": "string" },
    "version": { "type": "number" }
  },
  "name": "alice",
  "version": 1
}
```

And this is the same document in YAML:

```yaml
fields:
  name:
    type: string
  version:
    type: number
name: alice
version: 1
```

Both parse into the same `RuntimeValue`. Both validate the same way. The core, VM, and schema system don't know which frontend produced the value.

### Schema-agnostic

The `Schema<V>` trait is abstract. The current implementation uses JSON Schema (`ac-schema-json`), but any schema language can implement it:

```rust
pub trait Schema<V> {
    fn validate(&self, value: &V) -> Result<(), ValidationError>;
}
```

Swapping JSON Schema for another validator (Cue, custom rules, etc.) doesn't touch the core, VM, or frontends.

ac-lang is a configuration language, not a scripting language. All values are JSON-serializable — `RuntimeValue = serde_json::Value` and schemas define types.
