# Why SchemaVocabulary feels orthogonal

A dialect normally means _how the language is written_ — interpolation markers, path separators, call delimiters, literal syntax. `SchemaVocabulary` answers a different question: _what schema language do we use to describe and validate values?_

| Concern         | Question                         | Sub-trait              |
| --------------- | -------------------------------- | ---------------------- |
| Surface syntax  | How are AC expressions written?  | `Syntax`, `ExprParser` |
| Document keys   | What are the reserved key names? | `DocumentVocabulary`   |
| Schema language | How are schemas expressed?       | `SchemaVocabulary`     |

The first three are about the AC language itself. `SchemaVocabulary` is about a _separate language_ (JSON Schema) embedded inside AC documents.

## Why it's bundled anyway

The `Dialect` facade groups all four because they share a property: they are the _configuration the compiler consumes at every call site_. The compiler obtains schema validators through the dialect — `dialect.schema_vocabulary().compile(raw)` — just as it obtains path separators through `dialect.syntax().path_separator()`. Bundling them means call sites accept one `&dyn Dialect` rather than two separate parameters.

## The coupling to be conscious of

The bundle is a convenience, not a semantic claim. A dialect change (e.g. `${...}` → `{{...}}`) has nothing to do with schema language choice (e.g. JSON Schema → CUE). They are independent decisions that happen to share a delivery mechanism. If the coupling ever causes friction — for example, if schema vocabulary needs to vary independently of syntax — splitting `SchemaVocabulary` out of the `Dialect` facade is a clean refactor: it already has its own trait and accessor.
