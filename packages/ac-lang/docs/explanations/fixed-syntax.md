# Why syntax is not customizable

AC separates concerns into three layers:

- **Language** defines syntax (`.` for paths, `()` for calls, `${...}` for interpolation).
- **Runtime host** defines capabilities (which namespaces, which functions, which import sources).
- **Frontend** defines format (YAML, JSON, TOML — all map to the same JSON-compatible values).

Making the _syntax_ customizable would break this separation. The `DEFAULT_NAMESPACE` and `IMPORT_FUNCTION` constants make _which names exist_ configurable without changing _how the language works_ — that is the right kind of abstraction. Syntax customization would be a different kind entirely.

## Portability

If an AC document written for one runtime uses `doc.name` and another uses `doc/name`, documents aren't portable between runtimes. You'd have dialects, not a single language. The whole point of a language spec is that syntax is fixed — that's what makes it a language.

## Tooling

Every editor plugin, linter, syntax highlighter, and parser needs to know the exact syntax. Customizable syntax means every tool needs to be parameterized, or only works with one dialect.

## The AST is already the abstraction

If someone genuinely needs slash-based lookups, they can write a custom frontend that transforms `ns/key` → `ns.key` before feeding it to the existing parser. The expression AST (`Expr<V>` with `Value` and `Call`) is already generic — the syntax is just one encoding of it. That's the right place for customization, not the language spec.

## Precedent

No successful config or expression language does this. YAML, JSON, TOML, HCL, Dhall, Jsonnet — all define one syntax. Runtimes differ in _capabilities_, not _grammar_.
