# AC Dialect Specification

**Version:** 0.1.0
**Status:** Draft

The AC Language Specification ([LANG.md](./LANG.md)) fixes one canonical syntax. This document specifies the Dialect API — the interface that abstracts the three syntax decisions behind a single contract, so implementations can be structured around one source of truth rather than hardcoding tokens across dozens of functions.

The canonical dialect is the only shipped dialect. Custom dialects are architecturally possible but not exposed officially to users. See [fixed-syntax.md](./explanations/fixed-syntax.md) for the rationale.

## 1. Overview

A **dialect** is a set of syntax tokens, key names, and parsing rules for AC documents. It defines:

- **Interpolation markers** — how expressions are embedded in strings.
- **Path separator** — how namespace paths are segmented.
- **Call syntax** — how function calls are delimited.
- **Literal parsing** — how string, number, boolean, and null literals are recognized.
- **Special key names** — the reserved keys used in AC documents (`fields`, `functions`, `expr`, `arg`, `merge`, `inheritable`, `mutable`, `params`, `return`, `body`).
- **Schema vocabulary** — which schema language is used for field and function validation.

The dialect does **not** define:

- Which namespaces exist (runtime host decision — see [RUNTIME.md](./RUNTIME.md)).
- Which functions exist (runtime host decision).
- The document format (YAML, JSON, TOML — frontend decision).

## 2. The Dialect interface

A dialect provides the following operations:

### 2.1 Syntax tokens

| Operation              | Description                                    | Canonical value |
| ---------------------- | ---------------------------------------------- | --------------- |
| `interpolation_prefix` | Marks the start of an interpolation expression | `${`            |
| `interpolation_suffix` | Marks the end of an interpolation expression   | `}`             |
| `path_separator`       | Separates segments in a path lookup            | `.`             |
| `call_open`            | Opens a function call argument list            | `(`             |
| `call_close`           | Closes a function call argument list           | `)`             |

These tokens are the minimal syntax surface. Every parser, interpolator, and type inferencer in the implementation reads them from the dialect rather than hardcoding strings.

### 2.2 Parsing operations

| Operation              | Input                | Output                   | Description                                                                                              |
| ---------------------- | -------------------- | ------------------------ | -------------------------------------------------------------------------------------------------------- |
| `find_call_open`       | Expression string    | Position or none         | Finds the call-open token outside string literals. Used to distinguish function calls from path lookups. |
| `split_args`           | Argument list string | List of argument strings | Splits arguments by the separator (`,` in canonical), respecting nested call tokens and string literals. |
| `parse_string_literal` | Trimmed string       | Content or none          | Recognizes string literals (e.g. `"hello"` or `'hello'`) and returns the inner content.                  |
| `parse_number_literal` | Trimmed string       | Number or none           | Recognizes number literals (e.g. `42`, `1.5`, `-3`) and returns the value.                               |
| `parse_bool_literal`   | Trimmed string       | Boolean or none          | Recognizes boolean literals (e.g. `true`, `false`) and returns the value.                                |
| `is_null_literal`      | Trimmed string       | Boolean                  | Recognizes the null literal (e.g. `null`).                                                               |

All literal parsing is dialect-configurable. A dialect that uses `yes`/`no` for booleans or `nil` for null implements `parse_bool_literal` and `is_null_literal` accordingly. The value type system (see [LANG.md](./LANG.md) §3) defines the _output_ types — `bool`, `null`, `f64`, `string` — not the _input_ syntax. A dialect changes how literals are recognized; the values they produce remain the same.

### 2.3 Argument separator

The argument separator is implicitly ``,` in the canonical dialect. It is not a separate token on the interface —`split_args`encapsulates the splitting logic, including nesting awareness. This means a custom dialect could use a different separator (e.g.`;`) by implementing `split_args`accordingly, without exposing a separate`argument_separator` token.

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

- **`ac_field_keys()`** — all field-level AC keys (`expr`, `arg`, `merge`, `inheritable`, `mutable`), used for stripping during type generation.
- **`reserved_keys()`** — top-level keys that cannot be declared as field names (`fields`, `functions`).

### 2.5 Schema vocabulary

The dialect defines which schema vocabulary is used for field and function validation. The schema vocabulary is an abstract interface that compiles raw schema documents into validators:

| Operation | Input               | Output                | Description                                                    |
| --------- | ------------------- | --------------------- | -------------------------------------------------------------- |
| `id`      | None                | Vocabulary identifier | e.g. `"json-schema-draft-2020-12"`                             |
| `compile` | Raw schema document | Validator             | Compiles a schema into a `Schema<V>` that can validate values. |

The canonical dialect uses JSON Schema draft 2020-12. See [LANG.md](./LANG.md) §6.1 for the language-level specification of schemas.

## 3. Canonical dialect

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

### 3.1 Call detection

A string is a function call if it contains the call-open token outside string literals. Otherwise, it is a path lookup. This is the single discriminator — no grammar ambiguity.

### 3.2 Argument splitting

Arguments are split by ``, at depth 0 (outside nested calls and string literals). Nested calls increase depth; the call-close token decreases it. String literals are transparent to depth tracking.

### 3.3 String literal parsing

Both double-quoted (`"..."`) and single-quoted (`'...'`) forms are accepted. The inner content is returned verbatim — no escape sequences are processed at the dialect level. Escape handling, if any, is a runtime concern.

### 3.4 Number literal parsing

Numbers are parsed as IEEE 754 doubles. Integers (`42`) and floats (`1.5`) are the same type (see [LANG.md](./LANG.md) §3). Negative numbers (`-3`) are recognized.

## 4. Custom dialects

A custom dialect implements the same interface with different token values and parsing rules. For example:

| Token           | Canonical           | Hypothetical custom |
| --------------- | ------------------- | ------------------- |
| Interpolation   | `${...}`            | `{{...}}`           |
| Path separator  | `.`                 | `/`                 |
| Call syntax     | `fn(args)`          | `fn[args]`          |
| String literals | `"..."` or `'...''` | `"..."` only        |

### 4.1 What a custom dialect must preserve

A custom dialect changes **surface syntax** only. It must not change:

- **Semantics** — interpolation, path lookup, and function calls work identically. Only the tokens differ.
- **Value types** — JSON-compatible values, IEEE 754 doubles, boolean/null keywords.
- **Namespace concept** — named scopes for path lookups. The path separator changes, but the mechanism does not.
- **Default namespace shorthand** — bare `key` resolves to `default.key` regardless of separator.
- **Expression AST** — `Literal`, `Path`, `Call` variants. The parser produces the same AST; only the token recognition differs.

### 4.2 What a custom dialect may change

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

### 4.3 Portability

Documents written with one dialect are not portable to another. `doc.name` and `doc/name` are different strings that produce the same AST only if the dialect is changed. This is why the language spec fixes one syntax — see [fixed-syntax.md](./explanations/fixed-syntax.md).

## 5. Relationship to the language spec

| Concern                       | Defined by    | Document                                          |
| ----------------------------- | ------------- | ------------------------------------------------- |
| Expression syntax (canonical) | Language spec | [LANG.md](./LANG.md) §4                           |
| Schema vocabulary (canonical) | Language spec | [LANG.md](./LANG.md) §6.1                         |
| Special key names (canonical) | Language spec | [LANG.md](./LANG.md) §6                           |
| Dialect interface (abstract)  | This spec     | DIALECT.md                                        |
| Runtime configuration         | Runtime spec  | [RUNTIME.md](./RUNTIME.md)                        |
| Fixed-syntax rationale        | Explanation   | [fixed-syntax.md](./explanations/fixed-syntax.md) |

The language spec defines **what the canonical syntax is**. This spec defines **how implementations abstract that syntax behind an interface**. The runtime spec defines **how the canonical dialect is wired into the engine**.

## 6. Reference implementation

The Rust implementation provides:

- **`ac-dialect`** crate — defines the `Dialect` trait (the abstraction). No implementation, no dependencies on a specific schema vocabulary.
- **`ac-dialect-canonical`** crate — implements the canonical `AcDialect` struct and `AC_DIALECT` static singleton. Depends on `ac-schema-json` for the JSON Schema vocabulary.
- **`ac-schema`** crate — defines the `Schema<V>` and `SchemaVocabulary<V>` traits (abstract schema validation).
- **`ac-schema-json`** crate — implements `JsonSchema` (`Schema<V>`) and `JsonSchemaVocabulary` (`SchemaVocabulary<V>`) for JSON Schema draft 2020-12.

Every function that parses or evaluates expressions accepts `&dyn Dialect` (from `ac-dialect`). Every function that extracts fields reads key names from the dialect. Every function that validates schemas uses the dialect's `SchemaVocabulary`. The canonical `AC_DIALECT` (from `ac-dialect-canonical`) is passed at call sites. No function hardcodes `"${"`, `'.'`, `'('`, `"fields"`, `"expr"`, or `JsonSchema`.

See [RUNTIME.md](./RUNTIME.md) for how the dialect integrates into the engine pipeline.
