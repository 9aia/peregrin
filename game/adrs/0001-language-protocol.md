# ADR-0001: Language — the protocol

## Status

Proposed

## Context

The game requires a communication system between players and NPCs that is
semantically real — not keyword matching, not dialogue trees, but actual
language comprehension. AC-Lang provides the runtime foundation (see
Peregrin ADR-0034), but the game-specific design decisions about how the
language maps to gameplay need their own record.

The core insight: Expr trees are the semantic representation. Questions are
operations. Answers are continuations. This is not a metaphor — it is the
literal implementation.

## Decision

The conlang is the protocol by which all interaction with formal minds
occurs. The language layer is responsible for:

- **Expr trees as semantic representation.** Every utterance — spoken,
  typed, written, or gestured — becomes an `Expr` tree. The tree is the
  meaning. All downstream systems (psychology, economy, society, culture,
  memory, world) operate on the tree, not the surface form.

- **Questions are operations.** A question word (`ma`-equivalent) causes
  the VM to signal an `Operation` and suspend evaluation. The listener
  (NPC or player) receives the suspended `Continuation` and resumes it
  with a value. The answer is literally a resumed computation.

- **Answers are continuations.** When an NPC answers a question, they call
  `continuation.resume(value)`. The player's original expression completes
  with the provided value. Multi-turn discourse is a chain of suspended
  and resumed continuations.

- **Statements are pure evaluation.** A declarative utterance ("the
  glowstone is blue") evaluates to a value with no side effects. It only
  has consequences if downstream systems (memory, society) choose to
  observe and store it.

- **Commands are operations with side effects.** An imperative ("give me
  the sword") signals an operation that expects the world to change. The
  handler resumes with the result of the change (success, refusal, error).

- **Input is modality-agnostic.** Text, speech (STT), gestures, and
  written glyphs all produce the same `Expr` trees through different
  `Lexer` implementations. The language does not know which modality
  produced it.

- **Streaming for real-time interaction.** The lexer emits tokens as input
  arrives. The parser produces partial `Frame`s. NPCs can react to partial
  input before the utterance is complete.

- **Fuzzy matching for imperfect input.** The host's `function` lookup
  includes a fuzzy matching layer (Levenshtein distance, configurable
  threshold per NPC). Strict NPCs require exact words. Patient NPCs
  tolerate typos. Thresholds shrink as player proficiency grows.

- **The language is formal, not natural.** The grammar is unambiguous.
  The pipeline is binary: grammatical or not. This is the aesthetic of
  the game world — formal minds require formal language.

## Consequences

### Positive

- NPCs semantically understand the player. Comprehension is evaluation.
- Questions, statements, and commands have distinct formal semantics.
- Multi-turn conversation is continuation chaining — no dialogue trees.
- Any input modality works by implementing a `Lexer`.
- Streaming enables real-time NPC reactions mid-utterance.

### Negative

- Formal strictness: slightly invalid input fails without a fuzzy layer.
- Every word must be implemented as a host function — no free vocabulary.
- Streaming lexer and incremental parser must be built (do not exist yet).

## References

- Peregrin ADR-0034: AC-Lang as the foundation for a game conlang
- `Host` trait: `packages/ac-lang/crates/vm/src/host.rs`
- `Continuation`: `packages/ac-lang/crates/vm/src/continuation.rs`
- `Lexer` trait: `packages/ac-lang/crates/lexer/src/lib.rs`
- `Dialect` trait: `packages/ac-lang/crates/dialect/src/dialect/dialect.rs`
