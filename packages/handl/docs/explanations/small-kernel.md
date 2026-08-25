# Why the language kernel is small

The handl language kernel has six primitives:

| Primitive              | What it does                               |
| ---------------------- | ------------------------------------------ |
| `$:`                   | Declare schema for data                    |
| `expr:`                | Compute a field's value                    |
| `arg:`                 | Feed a field's data value into its `expr:` |
| `merge: true`          | Spread computed object into parent scope   |
| `${...}`               | Interpolate paths and function calls       |
| `handle(body) { ... }` | Intercept operations with handler clauses  |

The language's expressive power comes from **composition** of these primitives, not from adding keywords. Patterns that would require dedicated syntax in other languages emerge naturally from field declarations and host-provided functions.

The reference runtime provides these host primitives:

| Host primitive           | What it does                       |
| ------------------------ | ---------------------------------- |
| `import(path)`           | Load an external document          |
| `apply(fn, ...)`         | Apply a first-class function value |
| `if(cond, then, else)`   | Conditional evaluation (lazy)      |
| `and(a, b)` / `or(a, b)` | Short-circuit logic (lazy)         |

These are runtime host conventions, not language-level features. A runtime host may provide different functions or none at all.

## The `import` field

The language describes its own module system using its own field syntax. Four primitives compose to produce an `import` field — no `import` keyword needed:

```yaml
$:
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
- **`expr: "import(arg)"`** — call the `import` host function on that path
- **`type: object`** — the output must be an object (required by merge)
- **`merge: true`** — spread the object into the parent scope, remove the `import` field

The field name `import` is just a regular name — not a reserved keyword. The language bootstraps its own import system from four primitives + one host function.

## The `functions` field

Functions are emergent too. A function value is just data — a JSON object with `params`, optional `return` schema, and a `body` expression string. A function is identified by the presence of a `body` string field. The `apply` host primitive evaluates a function value's body with bound arguments. No `functions:` keyword needed:

```yaml
$:
  functions:
    type: object
  result:
    type: number
    expr: "double(5)"

functions:
  double:
    params:
      x: { type: number }
    return: { type: number }
    body: "mul(x, 2)"

# result → 10
```

How it works:

- **`functions` field** — a regular field holding an object whose values are function values (objects with a `body` field)
- **`double(5)`** — the evaluator resolves `double` via path lookup to a function value, then dispatches via `apply`
- **`apply(fn, 5)`** — the host primitive binds `x = 5` and evaluates the body `"mul(x, 2)"`

The field name `functions` is just a regular name — not a reserved keyword. User-defined functions emerge from field declarations + the `apply` host primitive.

## Other emergent patterns

- **`config` field** — `arg: { type: object }` + `expr: "arg"` + `merge: true`: spread a data object into scope without importing a file
- **Dynamic import** — compute the import path at runtime: `expr: "import(concat(arg, env.ENV))"` (see [LANG.md](../../docs/LANG.md) §8.3)
- **Function libraries** — `import()` a document with function values in its data; the functions are accessible via path resolution (e.g. `lib.functions.my_fn(arg)`)
- **Control flow** — `if`, `and`, `or` as lazy host functions; `map`, `filter`, `reduce` as regular host functions
- **Generic functions** — `typeParams` + `{ "type": "var", "name": "T" }` in function schemas enable parametric polymorphism, typechecked via unification — no runtime impact, no new keywords (see [LANG.md](../../docs/LANG.md) §9.6)
- **Error handling** — operations signal via `handle` expressions; handlers intercept, resume, or discard
- **Async I/O** — `AsyncFunction` trait for non-blocking network/timer operations via `eval_async`
- **Cross-machine resumability** — `eval_resumable` + serializable `Continuation` + `CacheHost` for server→client migration
- **Platform services** — `fs.read`, `env.get`, `http.get`, `cmd.run` via `Module` + `ModuleHost` composition

## The principle

The kernel stays small. The library of patterns grows. Users can invent their own field conventions without touching the compiler. The language is its own metalanguage — it describes its own extensions using its own field declaration syntax. Control flow, async I/O, cross-machine resumability, and platform services all emerge from the host + kernel split.
