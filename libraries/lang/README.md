# handl Language Library — Reusable field patterns

The `lang/` namespace holds reusable handl language field patterns. These are
not workflow artifacts — they are **field schema definitions** that emerge
from the language's own primitives (`expr`, `arg`, `merge`, `import()`).

## Patterns

| File               | Pattern        | Description                                                   |
| ------------------ | -------------- | ------------------------------------------------------------- |
| `import.field.yml` | `import` field | Load an external document and merge it into the current scope |

## The `import` field pattern

The handl language doesn't need an `import` keyword. Four primitives compose
to produce it:

- **`arg: { type: string }`** — the field's data value (a path string) is the input
- **`expr: "import(arg)"`** — call `import()` on that path
- **`type: object`** — the output must be an object (required by merge)
- **`merge: true`** — spread the object into the parent scope, remove the field

### Usage

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

The data declares `import: "./base.yml"`. The field schema says: take that
path string (`arg`), call `import()` on it, require the result to be an
object, then spread it into the parent and remove the `import` field.

### Scoped usage

```yaml
fields:
  database:
    type: object
    fields:
      import:
        type: object
        arg: { type: string }
        expr: "import(arg)"
        merge: true
      timeout:
        type: number

database:
  import: "./db_config.yml"
  timeout: 30
```

Imports `db_config.yml` and merges it **only into `database`**. The `timeout`
field in the parent is preserved (parent wins on conflicts).

### Why this works

The language is self-describing: its own primitives generate the import
pattern as emergent behavior. No compiler support needed — `arg:` bridges
data to expression, `import()` does the loading, `merge` flattens the result.
The field name `import` is just a regular name, not a reserved keyword.
