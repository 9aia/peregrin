# Why syntax is not customizable

handl separates concerns into three layers:

- **Language** defines syntax (`.` for paths, `()` for calls, `${...}` for interpolation).
- **Runtime host** defines capabilities (which namespaces, which functions, which import sources).
- **Frontend** defines format (YAML, JSON, TOML — all map to the same JSON-compatible values).

Making the _syntax_ customizable would break this separation. The `DEFAULT_NAMESPACE` and `IMPORT_FUNCTION` constants make _which names exist_ configurable without changing _how the language works_ — that is the right kind of abstraction. Syntax customization would be a different kind entirely.

## Portability

If an handl document written for one runtime uses `doc.name` and another uses `doc/name`, documents aren't portable between runtimes. You'd have dialects, not a single language. The whole point of a language spec is that syntax is fixed — that's what makes it a language.

## Tooling

Every editor plugin, linter, syntax highlighter, and parser needs to know the exact syntax. Customizable syntax means every tool needs to be parameterized, or only works with one dialect.

## The AST is already the abstraction

If someone genuinely needs slash-based lookups, they can write a custom frontend that transforms `ns/key` → `ns.key` before feeding it to the existing parser. The expression AST (`Expr<V>` with `Value` and `Call`) is already generic — the syntax is just one encoding of it. That's the right place for customization, not the language spec.

## Precedent

No successful config or expression language does this. YAML, JSON, TOML, HCL, Dhall, Jsonnet — all define one syntax. Runtimes differ in _capabilities_, not _grammar_.

## Internal dialect abstraction in the Rust implementation

The language spec fixes syntax. The Rust implementation, however, abstracts the syntax decisions, special key names, and schema vocabulary behind a `Dialect` trait in the `handl-dialect` crate. This is an implementation detail, not a language-level feature. The full dialect interface is specified in [SPEC.md](../../SPEC.md).

The canonical `HandlDialect` implements the spec syntax: `${...}` for interpolation, `.` for paths, `fn()` for calls, the canonical key names (`$`, `functions`, `expr`, `arg`, `merge`, `inheritable`, `mutable`, `params`, `return`, `body`), and the JSON Schema draft 2020-12 vocabulary. It is the only shipped dialect. The trait exists so the implementation is structured around a single source of truth, rather than hardcoding tokens and key names across dozens of functions.

This does not contradict the fixed-syntax principle. The spec says there is one syntax; the implementation says that syntax lives in one place. A custom dialect would produce documents that are not portable and tools that do not interoperate — the problems described above. The trait is not an extension point for users; it is an internal organizing principle.

## Internal schema abstraction in the Rust implementation

The same pattern applies to schema validation. The Rust implementation abstracts schema validation behind a `Schema<V>` trait and a `SchemaVocabulary<V>` trait that compiles raw schema documents into validators, both defined in the `handl-dialect` crate. The canonical implementation, `JsonSchemaVocabulary` in the `handl-dialect-canonical` crate, compiles raw schemas into `JsonSchema` validators using the `jsonschema` crate for full JSON Schema draft 2020-12 validation. It is the only shipped implementation.

The `Schema<V>` and `SchemaVocabulary<V>` traits are used generically throughout the compiler (e.g., function argument and return validation in `handl-fields`). The compiler never depends on a specific schema vocabulary directly — it obtains the vocabulary from the dialect's `schema_vocabulary()` method and uses the trait. `JsonSchemaVocabulary` is wired in through `HandlDialect`. This mirrors the `Dialect` trait: the spec fixes JSON Schema draft 2020-12 as the schema vocabulary; the implementation says that vocabulary lives behind one interface, accessed through the dialect. A custom `SchemaVocabulary<V>` implementation would be an internal organizing possibility, not a user extension point — the language spec expects JSON Schema, and only JSON Schema is exposed.
