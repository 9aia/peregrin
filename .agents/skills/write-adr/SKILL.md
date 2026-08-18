---
name: write-adr
description: How to write Architecture Decision Records for the Peregrin project. Use when making non-obvious design decisions, architecture changes, or banning patterns.
---

# Write an ADR

## When to write

- A non-obvious design choice is made.
- A pattern is banned or required.
- A dependency or tool changes.

## Structure

```
project/adrs/NNNN-title.md
  # Title
  ## Context
  ## Decision
  ## Consequences
  ## Status
```

## Steps

1. Create `project/adrs/NNNN-title.md`.
2. Explain context, decision, and consequences.
3. Mark status as `proposed`, `accepted`, `deprecated`, or `superseded`.
4. Reference the ADR from `rules/` or `.windsurfrules` if it introduces a rule.
