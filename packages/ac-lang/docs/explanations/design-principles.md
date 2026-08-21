# Design principles

1. **Declarative.** A document describes _what_ values it contains, not _how_ to compute them. Computation is expressed through field declarations, not imperative code.
2. **Schema-first.** Every field has a schema. Validation is not optional — it is part of the language semantics.
3. **Named functions are emergent.** The language defines no built-in functions and no `functions:` keyword. A function is just an object with a `body` field stored in a field. The `apply` host primitive evaluates function bodies with bound arguments. User-defined functions compose from host primitives and each other — no new language constructs needed.
4. **Composition over keywords.** The language has a small set of primitives. Complex patterns (imports, config spreading, conditional loading) emerge from composition, not from dedicated syntax.
5. **Format-agnostic.** Documents can be written in YAML, JSON, TOML, or any format that maps to JSON-compatible values.
