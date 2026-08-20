# AC Dialect Specification

**Status:** Draft

The AC Language Specification ([LANG.md](./LANG.md)) fixes one canonical syntax. This document specifies the Dialect API — the interface that abstracts the three syntax decisions behind a single contract, so implementations can be structured around one source of truth rather than hardcoding tokens across dozens of functions.

The canonical dialect is the only shipped dialect. Custom dialects are architecturally possible but not exposed officially to users. See [fixed-syntax.md](./explanations/fixed-syntax.md) for the rationale.

## 1. Overview

A **dialect** is a set of syntax tokens, key names, and parsing rules for AC documents. It defines:

- **Interpolation markers** — how expressions are embedded in strings (in canonical: `${...}`).
- **Path separator** — how namespace paths are segmented (in canonical: `.`).
- **Call syntax** — how function calls are delimited (in canonical: `fn(args)`).
- **Literal parsing** — how string, number, boolean, and null literals are recognized (in canonical: `"..."`, `42`, `true`, `null`).
- **Special key names** — the reserved keys used in AC documents (in canonical: `fields`, `functions`, `expr`, `arg`, `merge`, `inheritable`, `mutable`, `params`, `return`, `body`).
- **Schema vocabulary** — which schema language is used for field and function validation (in canonical: JSON Schema draft 2020-12).

The dialect does **not** define:

- Which namespaces exist (runtime host decision — see [RUNTIME.md](./RUNTIME.md)).
- Which functions exist (runtime host decision).
- The document format (YAML, JSON, TOML — frontend decision).

## 2. The Dialect interface

A dialect provides the following operations. All operations are methods on the dialect interface — in Rust, the `Dialect` trait; in other languages, an interface, protocol, or abstract class.

### 2.1 Syntax tokens

The interpolation markers are string tokens (may be multi-character). The path separator, call-open, and call-close are single-character tokens. In the Rust implementation, string tokens are `&str` and single-character tokens are `char`; in other languages, both may be represented as strings.

| Operation              | Token type  | Description                                    | Canonical value |
| ---------------------- | ----------- | ---------------------------------------------- | --------------- |
| `interpolation_prefix` | String      | Marks the start of an interpolation expression | `${`            |
| `interpolation_suffix` | String      | Marks the end of an interpolation expression   | `}`             |
| `path_separator`       | Single char | Separates segments in a path lookup            | `.`             |
| `call_open`            | Single char | Opens a function call argument list            | `(`             |
| `call_close`           | Single char | Closes a function call argument list           | `)`             |

These tokens are the minimal syntax surface. Every parser, interpolator, and type inferencer in the implementation reads them from the dialect rather than hardcoding strings.

### 2.1.1 Concurrency

Dialect implementations must be safe for concurrent access. In Rust, this is expressed via the `Send + Sync` supertraits on `Dialect` and `SchemaVocabulary`. In other languages, the equivalent concurrency safety guarantee should be applied — e.g. thread-safe interfaces in Java, `Send` protocols in Swift, or immutable/frozen objects in JavaScript.

### 2.2 Parsing operations

| Operation              | Input                | Output                   | Description                                                                                              |
| ---------------------- | -------------------- | ------------------------ | -------------------------------------------------------------------------------------------------------- |
| `find_call_open`       | Expression string    | Position or none         | Finds the call-open token outside string literals. Used to distinguish function calls from path lookups. |
| `split_args`           | Argument list string | List of argument strings | Splits arguments by the separator (`,` in canonical), respecting nested call tokens and string literals. |
| `parse_string_literal` | Trimmed string       | Content or none          | Recognizes string literals (in canonical: `"hello"` or `'hello'`) and returns the inner content.         |
| `parse_number_literal` | Trimmed string       | Number or none           | Recognizes number literals (in canonical: `42`, `1.5`, `-3`) and returns the value.                      |
| `parse_bool_literal`   | Trimmed string       | Boolean or none          | Recognizes boolean literals (in canonical: `true`, `false`) and returns the value.                       |
| `is_null_literal`      | Trimmed string       | Boolean                  | Recognizes the null literal (in canonical: `null`).                                                      |

All literal parsing is dialect-configurable. A dialect that uses `yes`/`no` for booleans or `nil` for null implements `parse_bool_literal` and `is_null_literal` accordingly. The value type system (see [LANG.md](./LANG.md) §3) defines the _output_ types — boolean, null, floating-point number, string — not the _input_ syntax. A dialect changes how literals are recognized; the values they produce remain the same.

### 2.3 Argument separator

The argument separator (`,` in canonical) is not a separate token on the interface. `split_args` encapsulates the splitting logic, including nesting awareness. A custom dialect could use a different separator (e.g. `;`) by implementing `split_args` accordingly, without exposing a separate `argument_separator` token.

### 2.4 Special key names

The dialect defines the reserved key names used in AC documents. These keys have special meaning during field extraction, type checking, and type generation:

| Key               | Level          | Purpose                                    | Canonical value |
| ----------------- | -------------- | ------------------------------------------ | --------------- |
| `fields_key`      | Top-level      | Field declarations                         | `fields`        |
| `functions_key`   | Top-level      | Function declarations                      | `functions`     |
| `expr_key`        | Field-level    | Computed field expression                  | `expr`          |
| `arg_key`         | Field-level    | Parameterized computed field argument type | `arg`           |
| `merge_key`       | Field-level    | Merge field flag                           | `merge`         |
| `inheritable_key` | Field-level    | Scoped inheritance flag                    | `inheritable`   |
| `mutable_key`     | Field-level    | Immutability flag                          | `mutable`       |
| `params_key`      | Function-level | Function parameters                        | `params`        |
| `return_key`      | Function-level | Function return schema                     | `return`        |
| `body_key`        | Function-level | User-defined function body                 | `body`          |

The dialect also provides two convenience aggregations:

- **`reserved_field_keys()`** — all field-level reserved keys (`expr`, `arg`, `merge`, `inheritable`, `mutable`), used for stripping during type generation.
- **`top_level_reserved_keys()`** — top-level keys that cannot be declared as field names (`fields`, `functions`).

### 2.5 Schema vocabulary

The dialect exposes its schema vocabulary through a `schema_vocabulary()` accessor on the `Dialect` interface, returning a reference to the vocabulary. The schema vocabulary is an abstract interface that compiles raw schema documents into validators:

At the dialect level, the value type `V` is fixed to the runtime's value type (in Rust, `RuntimeValue`). The `Schema<V>` and `SchemaVocabulary<V>` interfaces remain generic over `V` so the abstractions work with any value type. In other language implementations, `V` should be bound to the equivalent runtime value type.

| Operation           | Input               | Output                | Description                                                  |
| ------------------- | ------------------- | --------------------- | ------------------------------------------------------------ |
| `schema_vocabulary` | None                | Vocabulary reference  | Returns the dialect's schema vocabulary.                     |
| `id`                | None                | Vocabulary identifier | e.g. `"json-schema-draft-2020-12"`                           |
| `compile`           | Raw schema document | Validator             | Compiles a schema into a validator that can validate values. |

In the Rust implementation, the raw schema document is `serde_json::Value`. This ties the abstract interface to JSON as the serialization format, but does not tie it to a specific schema vocabulary — `serde_json` is a serialization library, not a schema language.

The canonical dialect uses JSON Schema draft 2020-12. See [LANG.md](./LANG.md) §6.1 for the language-level specification of schemas.

### 2.6 Validation interfaces

Schema validation involves two abstract interfaces and an error type, all generic over the value type `V`:

**`Schema<V>`** — a compiled validator that can validate a value:

| Operation  | Input        | Output                  | Description                                                                         |
| ---------- | ------------ | ----------------------- | ----------------------------------------------------------------------------------- |
| `validate` | Value of `V` | Ok or `ValidationError` | Validates a value against the compiled schema. Returns the first violation, if any. |

**`ValidationError`** — the error returned when validation fails:

| Field      | Type                     | Description                                                                                                                                                                                                         |
| ---------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `message`  | String                   | Human-readable description of the violation.                                                                                                                                                                        |
| `path`     | String                   | Dotted path to the offending value within the document (empty string for root).                                                                                                                                     |
| `location` | Optional source location | Source location (line:column) of the offending value, if known. In Rust, this is `Option<SourceLocation>` from the `ac-source` crate. In other languages, an equivalent source location type or `null`/`undefined`. |

`ValidationError` should be displayable (human-readable string representation) and comparable for equality. In Rust, it implements `Display`, `Error`, `Clone`, and `PartialEq`.

## 3. Package structure

The dialect abstraction and its canonical implementation are separate packages. This keeps the abstractions cacheable, testable and free of any dependency on a specific schema vocabulary or runtime.

| Package             | Role                            | Contents                                                                                                                                                                                           |
| ------------------- | ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dialect abstraction | Defines the interfaces          | `Dialect`, `Schema<V>`, `SchemaVocabulary<V>`, `ValidationError`. No implementation, no dependencies on a specific schema vocabulary.                                                              |
| Canonical dialect   | Implements the canonical syntax | `AcDialect` (implements `Dialect`), `AC_DIALECT` static singleton, `JsonSchema` (implements `Schema<V>`), `JsonSchemaVocabulary` (implements `SchemaVocabulary<V>`) for JSON Schema draft 2020-12. |

In Rust, the abstraction package is the `ac-dialect` crate and the canonical package is the `ac-dialect-canonical` crate. In other languages, the equivalent module/package split should be used — e.g. `@ac/dialect` and `@ac/dialect-canonical` in a JavaScript/TypeScript monorepo.

## 4. Canonical dialect

The canonical dialect implements the syntax defined in [LANG.md](./LANG.md):

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

### 4.1 Call detection

A string is a function call if it contains the call-open token outside string literals. Otherwise, it is a path lookup. This is the single discriminator — no grammar ambiguity.

### 4.2 Argument splitting

Arguments are split by `,` at depth 0 (outside nested calls and string literals). Nested calls increase depth; the call-close token decreases it. String literals are transparent to depth tracking.

### 4.3 String literal parsing

Both double-quoted (`"..."`) and single-quoted (`'...'`) forms are accepted. The inner content is returned verbatim — no escape sequences are processed at the dialect level. Escape handling, if any, is a runtime concern.

### 4.4 Number literal parsing

Numbers are parsed as IEEE 754 doubles. Integers (`42`) and floats (`1.5`) are the same type (see [LANG.md](./LANG.md) §3). Negative numbers (`-3`) are recognized.

## 5. Custom dialects

A custom dialect implements the same interface with different token values and parsing rules. For example:

| Token           | Canonical          | Hypothetical custom |
| --------------- | ------------------ | ------------------- |
| Interpolation   | `${...}`           | `{{...}}`           |
| Path separator  | `.`                | `/`                 |
| Call syntax     | `fn(args)`         | `fn args`           |
| String literals | `"..."` or `'...'` | `"..."` only        |

### 5.1 What a custom dialect must preserve

A custom dialect changes **surface syntax** only. It must not change:

- **Semantics** — interpolation, path lookup, and function calls work identically. Only the tokens differ.
- **Value types** — JSON-compatible values, IEEE 754 doubles, boolean/null values.
- **Namespace concept** — named scopes for path lookups. The path separator changes, but the mechanism does not.
- **Default namespace shorthand** — bare `key` resolves to `doc.key` (in canonical) regardless of separator.
- **Expression AST** — `Literal`, `Path`, and `Call` node types. The parser produces the same AST; only the token recognition differs.

### 5.2 What a custom dialect may change

- Interpolation markers (prefix/suffix)
- Path separator character
- Call delimiters (open/close)
- Argument separator (within `split_args`)
- String literal quote characters
- Number literal format (within `parse_number_literal`)
- Boolean literal keywords (within `parse_bool_literal`)
- Null literal keyword (within `is_null_literal`)
- Special key names (e.g. `fields` → `properties`, `expr` → `compute`)
- Schema vocabulary (e.g. a non-JSON-Schema validator)

### 5.3 Portability

Documents written with one dialect are not portable to another. `doc.name` and `doc/name` are different strings that produce the same AST only if the dialect is changed. This is why the language spec fixes one syntax — see [fixed-syntax.md](./explanations/fixed-syntax.md).

## 6. Relationship to the other specs

| Concern                       | Defined by    | Document                                          |
| ----------------------------- | ------------- | ------------------------------------------------- |
| Expression syntax (canonical) | Language spec | [LANG.md](./LANG.md) §4                           |
| Schema vocabulary (canonical) | Language spec | [LANG.md](./LANG.md) §6.1                         |
| Special key names (canonical) | Language spec | [LANG.md](./LANG.md) §6                           |
| Dialect interface (abstract)  | This spec     | DIALECT.md                                        |
| Runtime configuration         | Runtime spec  | [RUNTIME.md](./RUNTIME.md)                        |
| Fixed-syntax rationale        | Explanation   | [fixed-syntax.md](./explanations/fixed-syntax.md) |

The language spec defines **what the canonical syntax is**. This spec defines **how implementations abstract that syntax behind an interface**. The runtime spec defines **how the canonical dialect is wired into the compiler**.

---

## 7. Implementation principle

Every function that parses or evaluates expressions should accept a dialect instance rather than hardcoding syntax tokens. Every function that extracts fields should read key names from the dialect. Every function that validates schemas should use the dialect's schema vocabulary. The canonical dialect instance is passed at call sites. No function should hardcode `"${"`, `'.'`, `'('`, `"fields"`, `"expr"`, or a specific schema implementation.

---

See [RUNTIME.md](./RUNTIME.md) for how the dialect integrates into the compiler.
