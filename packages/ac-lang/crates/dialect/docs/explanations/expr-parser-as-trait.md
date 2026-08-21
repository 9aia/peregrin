# Why ExprParser is a trait, not configuration

`Syntax` is static configuration — five token values, no logic. `ExprParser` is a trait with six methods that contain real parsing behavior. This split is deliberate.

## The problem

If a dialect can change:

- argument separators (`,` → `;`)
- nesting rules (which tokens open/close depth)
- quote syntax (`"..."` and `'...'` → `"..."` only)
- call delimiters (`fn(args)` → `fn args`)
- literal syntax (`true`/`false` → `yes`/`no`, `null` → `nil`)

then expression parsing cannot be reduced to static configuration. Splitting arguments by `,` at depth 0 is not a parameter — it is an algorithm. A dialect that uses `;` as separator and `[`/`]` as nesting tokens needs different splitting logic, not different constants fed into the same loop.

## The distinction

- **Syntax** describes _what the tokens are_ — the minimal set of string and character values every parser reads.
- **ExprParser** defines _how those tokens are interpreted_ — the algorithms that turn raw expression strings into structural decisions.

This makes the split clean:

```
Dialect
├── Syntax              → "what are the tokens?"
├── ExprParser          → "how do I parse them?"
├── DocumentVocabulary  → "what are the reserved keys?"
└── SchemaVocabulary    → "how are schemas interpreted?"
```

## Why not merge them

Merging `Syntax` into `ExprParser` would lose the separation between token values and parsing algorithms. Call sites that only need to know the interpolation prefix (e.g. a string interpolator) would depend on the full parsing interface. Call sites that only need `split_args` (e.g. an argument extractor) would also carry interpolation markers they never use.

Keeping `Syntax` as a thin, pure-data trait means consumers can depend on exactly the level of detail they need.

## What the canonical implementation does

The canonical `ExprParser` implementation in `ac-dialect-canonical` hardcodes the canonical algorithms: scan for `(` outside string literals, split by `,` at depth 0, recognize `"..."` and `'...'`, parse `f64`, match `true`/`false`/`null`. These are the canonical parsing rules — they are behavior, not configuration.

A hypothetical custom dialect would implement the same trait with different algorithms. The trait is the contract; the behavior is the implementation.
