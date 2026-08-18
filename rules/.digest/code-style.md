# Code Style (Digest)

- Explicit over clever. Use `pipe` and `Effect.gen`, not nested calls.
- Functions are verbs, booleans are predicates (`is_`, `has_`), types are nouns.
- No abbreviations: `config` not `cfg`, `instance` not `inst`.
- Use `Match` or `Option.match` for unions. Avoid catch-all `_`.
- Branded types with `Schema.Brand` to prevent domain mixups.
- Early returns. Max 3 levels of nesting.
- Functions ~30 lines. Don't extract 1-2 line helpers unless they add clarity.
- Delete commented-out code. Git remembers.
