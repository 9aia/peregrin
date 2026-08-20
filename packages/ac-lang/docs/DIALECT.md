# AC Dialect Specification

**Version:** 0.1.0
**Status:** Draft

The AC Language Specification ([LANG.md](./LANG.md)) fixes one canonical syntax. This document specifies the Dialect API — the interface that abstracts the three syntax decisions behind a single contract, so implementations can be structured around one source of truth rather than hardcoding tokens across dozens of functions.

The canonical dialect is the only shipped dialect. Custom dialects are architecturally possible but not exposed to users. See [fixed-syntax.md](./explanations/fixed-syntax.md) for the rationale.

## 1. Overview

A **dialect** is a set of syntax tokens and parsing rules for AC expressions. It defines:

- **Interpolation markers** — how expressions are embedded in strings.
- **Path separator** — how namespace paths are segmented.
- **Call syntax** — how function calls are delimited.
- **Literal parsing** — how string, number, boolean, and null literals are recognized.

The dialect does **not** define:

- Which namespaces exist (runtime host decision — see [RUNTIME.md](./RUNTIME.md)).
- Which functions exist (runtime host decision).
- The document format (YAML, JSON, TOML — frontend decision).
- The schema vocabulary (JSON Schema draft 2020-12 — see [LANG.md](./LANG.md) §6.1).

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

Boolean literals (`true`, `false`) and the null literal (`null`) are recognized as keywords — they are not dialect-configurable. A dialect that changed `true` to `yes` would break the value type system (see [LANG.md](./LANG.md) §3).

### 2.3 Argument separator

The argument separator is implicitly ``,` in the canonical dialect. It is not a separate token on the interface —`split_args`encapsulates the splitting logic, including nesting awareness. This means a custom dialect could use a different separator (e.g.`;`) by implementing `split_args`accordingly, without exposing a separate`argument_separator` token.

## 3. Canonical dialect

The canonical dialect implements the syntax defined in [LANG.md](./LANG.md):

| Token              | Value              | Used in                                                 |
| ------------------ | ------------------ | ------------------------------------------------------- |
| Interpolation      | `${...}`           | String interpolation, `expr:` fields, `body:` functions |
| Path separator     | `.`                | `ns.key`, `ns.key.sub`, shorthand `key`                 |
| Call syntax        | `fn(args)`         | `add(1, 2)`, `mul(x, 2)`, `concat(doc.name, "!")`       |
| String literals    | `"..."` or `'...'` | `"hello"`, `'hello'`                                    |
| Number literals    | `42`, `1.5`, `-3`  | IEEE 754 doubles                                        |
| Boolean literals   | `true`, `false`    | Keywords (not dialect-configurable)                     |
| Null literal       | `null`             | Keyword (not dialect-configurable)                      |
| Argument separator | `,`                | `add(1, 2, 3)`                                          |

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

### 4.3 Portability

Documents written with one dialect are not portable to another. `doc.name` and `doc/name` are different strings that produce the same AST only if the dialect is changed. This is why the language spec fixes one syntax — see [fixed-syntax.md](./explanations/fixed-syntax.md).

## 5. Relationship to the language spec

| Concern                       | Defined by    | Document                                          |
| ----------------------------- | ------------- | ------------------------------------------------- |
| Expression syntax (canonical) | Language spec | [LANG.md](./LANG.md) §4                           |
| Dialect interface (abstract)  | This spec     | DIALECT.md                                        |
| Runtime configuration         | Runtime spec  | [RUNTIME.md](./RUNTIME.md)                        |
| Fixed-syntax rationale        | Explanation   | [fixed-syntax.md](./explanations/fixed-syntax.md) |

The language spec defines **what the canonical syntax is**. This spec defines **how implementations abstract that syntax behind an interface**. The runtime spec defines **how the canonical dialect is wired into the engine**.

## 6. Reference implementation

The Rust implementation provides:

- **`ac-dialect`** crate — defines the `Dialect` trait and the canonical `AcDialect` struct.
- **`AcDialect`** — the only shipped dialect. Implements all canonical tokens and parsing rules.
- **`AC_DIALECT`** — a static singleton of `AcDialect`, used throughout the engine.

Every function that parses or evaluates expressions accepts `&dyn Dialect`. The canonical `AC_DIALECT` is passed at call sites. No function hardcodes `"${"`, `'.'`, or `'('`.

See [RUNTIME.md](./RUNTIME.md) for how the dialect integrates into the engine pipeline.
