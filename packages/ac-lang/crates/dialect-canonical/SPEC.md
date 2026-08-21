# AC Canonical Dialect Specification

**Status:** Approved

This document specifies the canonical dialect — the default and only shipped implementation of the [Dialect interface](../dialect/SPEC.md). The dialect spec defines the abstract interface; this spec defines the concrete token values, parsing rules, and schema vocabulary for the canonical syntax.

The canonical dialect implements the syntax defined in [LANG.md](../../docs/LANG.md):

| Token              | Value                                                                                  | Used in                                                 |
| ------------------ | -------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| Interpolation      | `${...}`                                                                               | String interpolation, `expr:` fields, `body:` functions |
| Path separator     | `.`                                                                                    | `ns.key`, `ns.key.sub`, shorthand `key`                 |
| Call syntax        | `fn(args)`                                                                             | `add(1, 2)`, `mul(x, 2)`, `concat(doc.name, "!")`       |
| String literals    | `"..."` or `'...'`                                                                     | `"hello"`, `'hello'`                                    |
| Number literals    | `42`, `1.5`, `-3`                                                                      | IEEE 754 doubles                                        |
| Boolean literals   | `true`, `false`                                                                        | Dialect-configurable (`parse_bool_literal`)             |
| Null literal       | `null`                                                                                 | Dialect-configurable (`is_null_literal`)                |
| Argument separator | `,`                                                                                    | `add(1, 2, 3)`                                          |
| Schema vocabulary  | JSON Schema 2020-12                                                                    | Field and function validation                           |
| Special keys       | `fields`, `expr`, `arg`, `merge`, `inheritable`, `mutable`, `params`, `return`, `body` | Document structure                                      |

## 1. Call detection

A string is a function call if it contains the call-open token (`(`) outside string literals. Otherwise, it is a path lookup. This discriminator is defined in [LANG.md](../../docs/LANG.md) §4.3: "If the expression contains `(`, it's a function call. Otherwise, it's a path lookup."

## 2. Argument splitting

Arguments are split by `,` at depth 0 (outside nested calls and string literals). Nested calls increase depth; the call-close token (`)`) decreases it. String literals are transparent to depth tracking.

## 3. String literal parsing

Both double-quoted (`"..."`) and single-quoted (`'...'`) forms are accepted. The inner content is returned verbatim — no escape sequences are recognized or processed. LANG.md does not define any escape syntax for string literals; a backslash inside a quoted string is a literal backslash, not an escape prefix.

## 4. Number literal parsing

Numbers are parsed as IEEE 754 doubles. Integers (`42`) and floats (`1.5`) are the same type (see [LANG.md](../../docs/LANG.md) §3). Negative numbers (`-3`) are recognized.

## 5. Implementation

The canonical dialect implementation provides:

- A **dialect type** that implements `Syntax`, `ExprParser`, `DocumentVocabulary`, and `Dialect`.
- A **dialect singleton** — a static instance of the dialect type for use at call sites.
- A **schema type** that implements `Schema<V>` using the canonical schema vocabulary.
- A **schema vocabulary type** that implements `SchemaVocabulary<V>` for JSON Schema draft 2020-12.
- A **schema vocabulary singleton** — a static instance of the schema vocabulary type.

### 5.1 Rust implementation

In Rust, the canonical dialect is implemented in the `ac-dialect-canonical` crate. It provides:

- `AcDialect` — implements `Syntax`, `ExprParser`, `DocumentVocabulary`, and `Dialect`
- `AC_DIALECT` — static singleton instance of `AcDialect`
- `JsonSchema` — implements `Schema<RuntimeValue>` for JSON Schema validation
- `JsonSchemaVocabulary` — implements `SchemaVocabulary<RuntimeValue>` for JSON Schema draft 2020-12
- `JSON_SCHEMA_VOCABULARY` — static singleton instance of `JsonSchemaVocabulary`

In other languages, the equivalent module/package split should be used — e.g. `@ac/dialect-canonical` in a JavaScript/TypeScript monorepo.

### 5.2 Module organization

The canonical implementation package is internally split into modules that mirror the sub-traits from the [Dialect interface](../dialect/SPEC.md). Each module owns one implementation:

| Module                | Contents                                                          |
| --------------------- | ----------------------------------------------------------------- |
| `syntax`              | `Syntax` implementation for the dialect type                      |
| `expr_parser`         | `ExprParser` implementation for the dialect type                  |
| `document_vocabulary` | `DocumentVocabulary` implementation for the dialect type          |
| `schema_vocabulary`   | `JsonSchema`, `JsonSchemaVocabulary`, `JSON_SCHEMA_VOCABULARY`    |
| `dialect`             | `AcDialect`, `AC_DIALECT` — facade that aggregates the four impls |

In Rust, each module is a directory with a `mod.rs` re-exporting the public API. In other languages, the equivalent module/file split should be used.

---

See [RUNTIME.md](../../docs/RUNTIME.md) for how the canonical dialect integrates into the compiler.
