# Why Dialect is split into four sub-traits

`Dialect` is a facade with four accessors. Each accessor returns a reference to a focused sub-trait. Consumers that need one aspect depend on the sub-trait; consumers that need multiple aspects use the facade.

## The four sub-traits

| Sub-trait             | Responsibility                                         | Kind                                 |
| --------------------- | ------------------------------------------------------ | ------------------------------------ |
| `Syntax`              | Interpolation markers, path separator, call delimiters | Static configuration (token values)  |
| `ExprParser`          | Call detection, argument splitting, literal parsing    | Behavior (parsing algorithms)        |
| `DocumentVocabulary`  | Reserved key names (`fields`, `expr`, etc.)            | Static configuration (string values) |
| `SchemaVocabulary<V>` | Schema language for validation                         | Behavior (compile + validate)        |

## Why not a single trait

A single `Dialect` trait with all methods would force every consumer to depend on the full surface, even when they need one aspect. A string interpolator only needs `Syntax` — the interpolation prefix and suffix. An argument extractor only needs `ExprParser`. A field extractor only needs `DocumentVocabulary`. A schema validator only needs `SchemaVocabulary<V>`.

With sub-traits, each consumer depends on exactly what it uses. The facade exists for call sites that genuinely need the whole configuration — the compiler entry point, for example, wires all four aspects together.

## Why the facade still exists

The facade is a convenience, not a requirement. It bundles the four sub-traits behind a single `&dyn Dialect` reference so call sites that need multiple aspects don't accept four separate parameters. The facade does not add behavior — it only groups accessors.

## Why the abstraction has no default implementations

Every method on every sub-trait is required. There are no default methods that return canonical values. This ensures the abstraction crate (`ac-dialect`) has zero dependency on any specific syntax, key names, or schema vocabulary. The canonical implementation lives entirely in `ac-dialect-canonical`. If defaults existed, the abstraction would implicitly encode the canonical dialect, defeating the purpose of the split.
