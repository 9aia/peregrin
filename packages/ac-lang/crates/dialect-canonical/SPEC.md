# AC Canonical Dialect Specification

**Status:** Draft

This document specifies the canonical dialect — the default and only shipped implementation of the [Dialect interface](../dialect/SPEC.md). The dialect spec defines the abstract interface; this spec defines the concrete token values, parsing rules, and schema vocabulary for the canonical syntax.

The canonical dialect implements the syntax defined in [LANG.md](../../docs/LANG.md):

| Token              | Value                                                                                               | Used in                                                 |
| ------------------ | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| Interpolation      | `${...}`                                                                                            | String interpolation, `expr:` fields, `body:` functions |
| Path separator     | `.`                                                                                                 | `ns.key`, `ns.key.sub`, shorthand `key`                 |
| Call syntax        | `fn(args)`                                                                                          | `add(1, 2)`, `mul(x, 2)`, `concat(doc.name, "!")`       |
| String literals    | `"..."` or `'...'`                                                                                  | `"hello"`, `'hello'`                                    |
| Number literals    | `42`, `1.5`, `-3`                                                                                   | IEEE 754 doubles                                        |
| Boolean literals   | `true`, `false`                                                                                     | Dialect-configurable (`parse_bool_literal`)             |
| Null literal       | `null`                                                                                              | Dialect-configurable (`is_null_literal`)                |
| Argument separator | `,`                                                                                                 | `add(1, 2, 3)`                                          |
| Schema vocabulary  | JSON Schema 2020-12                                                                                 | Field and function validation                           |
| Special keys       | `fields`, `functions`, `expr`, `arg`, `merge`, `inheritable`, `mutable`, `params`, `return`, `body` | Document structure                                      |

## 1. Call detection

A string is a function call if it contains the call-open token (`(`) outside string literals. Otherwise, it is a path lookup. This is the single discriminator — no grammar ambiguity.

## 2. Argument splitting

Arguments are split by `,` at depth 0 (outside nested calls and string literals). Nested calls increase depth; the call-close token (`)`) decreases it. String literals are transparent to depth tracking.

## 3. String literal parsing

Both double-quoted (`"..."`) and single-quoted (`'...'`) forms are accepted. The inner content is returned verbatim — no escape sequences are processed at the dialect level. Escape handling, if any, is a runtime concern.

## 4. Number literal parsing

Numbers are parsed as IEEE 754 doubles. Integers (`42`) and floats (`1.5`) are the same type (see [LANG.md](../../docs/LANG.md) §3). Negative numbers (`-3`) are recognized.

## 5. Implementation

The canonical dialect is implemented in the `ac-dialect-canonical` crate. It provides:

- `AcDialect` — implements `Syntax`, `ExprParser`, `DocumentVocabulary`, and `Dialect`
- `AC_DIALECT` — static singleton instance
- `JsonSchema` — implements `Schema<V>` for JSON Schema validation
- `JsonSchemaVocabulary` — implements `SchemaVocabulary<V>` for JSON Schema draft 2020-12
- `JSON_SCHEMA_VOCABULARY` — static singleton instance of `JsonSchemaVocabulary`

---

See [RUNTIME.md](../../docs/RUNTIME.md) for how the canonical dialect integrates into the compiler.
