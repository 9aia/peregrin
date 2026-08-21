# Why the language kernel is small

The AC language kernel has seven primitives:

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

## The `import` field

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

## Other emergent patterns

- **`config` field** — `arg: { type: object }` + `expr: "arg"` + `merge: true`: spread a data object into scope without importing a file
- **Dynamic import** — compute the import path at runtime: `expr: "import(concat(arg, env.ENV))"` (see [LANG.md](../../docs/LANG.md) §8.3)

## The principle

The kernel stays small. The library of patterns grows. Users can invent their own field conventions without touching the compiler. The language is its own metalanguage — it describes its own extensions using its own field declaration syntax.
