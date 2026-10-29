+++
id = "ADR-0075"
title = "Modular AST Layering"
status = "open"
priority = "medium"
+++

# Modular AST Layering

## Context

The `handl` workspace had a single `handl-ast` crate that mixed two responsibilities:

1. Generic, language-agnostic AST machinery (`Node<K, V, M>`, metadata, factory traits).
2. The handluage canonical AST (`Expr<V>`, `HandlerClause<V>`, `Field<V>`).

This made the crate neither fully reusable for other dialects nor clearly organized by domain. The factory traits for expressions, statements, and types were grouped in one file, and the canonical language types were coupled to the generic node structure. As the language grows, we need a clear boundary between reusable AST primitives and the handl-specific representation.

## Decision

Split AST code into three focused layers:

1. **`handl-ast-common`**: a reusable, language-agnostic AST core library.
   - No external dependencies.
   - Organized by domain: `node`, `metadata`, `expr`, `stmt`, `type_`.
   - Defines generic `Node<K, V, M>`, `NodeMetadata`, `Span`, `Attrs<A>`, `Both<M1, M2>`, and factory traits (`ExprFactory`, `StmtFactory`, `TypeFactory`).
   - Remains generic over value type `V` and metadata `M`.

2. **`handl-ast-canonical`**: the handluage canonical AST, built on `handl-ast-common`.
   - Depends only on `handl-ast-common`.
   - Organized by domain: `expr` and `field`.
   - Contains `Expr<V>`, `HandlerClause<V>`, `Field<V>`, and `CanonicalExprFactory`.
   - Re-exports the common machinery so consumers can depend on one crate.

3. **`handl-ast`**: a thin facade crate.
   - Re-exports both `handl-ast-common` and `handl-ast-canonical`.
   - Preserves the existing `use handl_ast::{Expr, Field, HandlerClause, ...}` API so no consumer needs to change.

`type` is a Rust keyword, so the type-domain module is named `type_`. This keeps the code valid and avoids raw identifiers in paths. If a better name becomes necessary, it can be renamed without touching consumers because it is an internal module in `handl-ast-common`.

## Consequences

- **Composable and decoupled**: `handl-ast-common` can be reused by other languages or dialects without pulling in handl semantics.
- **Modular by domain**: each domain (expressions, statements, types, nodes, metadata) has its own module.
- **Extensible**: new canonical constructs (statements, types, patterns) can be added to `handl-ast-canonical` without expanding `handl-ast-common`.
- **Agnostic and generic**: the core stays generic over `V` and `M`; only `handl-ast-canonical` pins the handl representation.
- **Backwards-compatible**: existing consumers import from `handl-ast` exactly as before.
- **Trade-off**: `handl-ast` is now a pass-through crate. If consumers migrate to `handl-ast-canonical` directly, `handl-ast` can later be deprecated or removed.

## Status

accepted
