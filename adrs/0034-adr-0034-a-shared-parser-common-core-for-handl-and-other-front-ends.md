+++
id = "ADR-0034"
title = "ADR-0034: A shared `parser-common` core for handl and other front-ends"
status = "open"
priority = "medium"
+++

# ADR-0034: A shared `parser-common` core for handl and other front-ends

## Status

Proposed

## Context

Peregrin is growing several language front-ends: the workflow language behind `handl`, template expressions in `handl-template-engine`, and future command-line or policy syntaxes. Each of these needs a tokenizer and a parser that builds an AST, but we currently have no shared foundation. A parser for one language should not copy another, and the parser core should not assume anything about a specific token set or node shape.

We also need to support different parsing strategies for the same language: top-down recursive descent for statements and Pratt operator-precedence for expressions. Writing a separate parser for each style makes the codebase harder to maintain and test.

The core must:

- Be agnostic to token kinds and AST node shapes.
- Be composable and extensible through a factory.
- Support both recursive descent and Pratt parsing in one runtime.
- Use the workspace's `Effect` conventions for errors instead of throwing.
- Be small enough that it can be reused by many packages without dragging in a full third-party parser-combinator library.

## Decision

Create `packages/parser-common`, a shared library that defines an agnostic parser runtime and AST model.

### Package shape

`packages/parser-common` is a library package with no dependencies on language-specific code. It exports the following public pieces:

- `Token<T>`, `Span`, and `Position` — a token model that any tokenizer can satisfy.
- `Node<K>` and `Ast<N>` — an AST model that any language can extend by supplying its own `kind` types.
- `ParseError` — a typed `Effect` error for syntax failures.
- `ParserContext<T, N>` — the runtime parser state plus primitive combinators:
  - `peek`, `advance`, `match`, `expect`, `isAtEnd` for recursive descent.
  - `expression(minBindingPower)` for Pratt operator-precedence parsing.
- `Grammar<T, N>` — a language-defined configuration that supplies:
  - `rules` keyed by token kind, each declaring optional `nud`/`led` handlers and binding powers.
  - `parseRoot` to start a parse and use either the recursive-descent primitives or `expression`.
- `createParser(grammar)` — a factory that returns a `parse(tokens, source?) => Effect<Ast<N>, ParseError>` function.

### Why this design

- The factory keeps the core decoupled from any grammar. `parser-common` knows how to drive a parse loop; the caller owns what each token means.
- `Token` and `Node` are type parameters, so the same engine can parse workflow YAML-style languages, template expressions, or a future Lisp/DSL without changes to the core.
- `expression` implements a standard Pratt parser using the `nud` (null denotation) and `led` (left denotation) rules supplied by the caller. `parseRoot` can call it or ignore it, so recursive descent and Pratt live side by side.
- All effects return `Effect<A, ParseError>`, aligning with the workspace's "no throw" and "Effect for expected errors" rules.
- Parser state is a `Ref` created once per `parse` call, isolating runs from each other.

### Boundaries

The core does not include:

- A tokenizer. Tokenizers live in each language package and produce `Token<T>` arrays.
- A grammar for any existing language. Grammars are owned by `handl`, `handl-template-engine`, or future consumers.
- Concrete expression/stmt AST nodes. It only defines the base `Node` and `Ast` shapes.
- Pretty-printing, formatting, or serialization. Those are separate packages or features.

## Consequences

- New language front-ends can reuse the same parser runtime and only define their token kinds, AST node subtypes, and grammar rules.
- Pratt and recursive descent can coexist in the same grammar without two separate parser implementations.
- Parse errors are typed values carried by `Effect`, so callers can short-circuit cleanly or collect diagnostics.
- The package depends on `effect` but adds no language-specific dependencies, keeping `packages` cleanly layered.
- Because parser state is created per call, the engine is safe to run concurrently.
- Performance of an `Effect`-based Pratt loop is acceptable for language parsing but should be benchmarked before it is used on very large inputs.
- We need to add `packages/parser-common` to the workspace catalog and keep it free of `apps` or language-specific code.

## Alternatives considered

- **Adopt a third-party parser combinator library.** Rejected. It would introduce external conventions, mismatch the workspace's `Effect` error model, and likely force a particular style of grammar definition.
- **Embed a parser in each language package (`handl`, `handl-template-engine`).** Rejected. It would duplicate the recursive descent and Pratt machinery and make it hard to keep error handling consistent.
- **Generate parsers from a grammar file.** Rejected as a first step. A programmable, data-driven factory is easier to evolve and lets language packages define grammars in TypeScript while we experiment with the right surface syntax.
