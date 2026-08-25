# ADR-0005: Culture — the drift

## Status

Proposed

## Context

A game world where every NPC speaks the same language with the same
vocabulary is static. Real languages diverge — different regions develop
different words, meanings shift over time, and contact between cultures
causes borrowing and mutation. The game needs cultural divergence to make
the world feel large and lived-in.

The `Dialect` trait and per-host `ModuleHost` architecture provide the
mechanism: different hosts can have different vocabularies, and the
`Dialect` trait allows different parsers for different regions. Words are
functions, and functions can be re-registered with different behavior.

## Decision

Culture is modeled as vocabulary divergence and semantic drift across
hosts:

- **Same dialect, different vocabularies.** All regions share the same
  grammar (the `Dialect`), but each region's NPCs have different modules
  registered on their hosts. The northern village knows `barter`, `furs`,
  `snow`. The southern city knows `exchange`, `spice`, `harbor`. Same
  grammar, different lexicons.

- **Words mutate as they propagate.** When a word travels from one culture
  to another (through trade, conquest, or migration), the receiving
  culture re-registers the function — possibly with different behavior:
  - **Broadening** — `glowstone` (originally a specific crafted item)
    becomes `glowstone` (any glowing stone) in the south
  - **Narrowing** — `trade` (originally any exchange) becomes `trade`
    (only formal commercial exchange) in a culture with strict commerce
    laws
  - **Shift** — `gift` (originally "to give freely") shifts to `gift`
    ("to poison") in a culture where gift-giving was used as a weapon

- **Etymology is registration metadata.** Each function registration
  records:
  - **Timestamp** — when the word entered this host
  - **Source** — who taught it (player, NPC, culture contact)
  - **Original meaning** — the function behavior at registration time
  - **Current meaning** — the function behavior now (may differ if
    re-registered)

  The player can query this through the meta-language: "where does
  'barter' come from?" → the host returns the registration metadata.

- **Word death is deregistration.** A culture that forgets a concept has
  the function removed from its hosts. The word stops working. The player
  says it, and the NPC doesn't understand. Causes of word death:
  - **Cultural collapse** — a village is destroyed, its hosts go offline
  - **Censorship** — a tyrant deregisters forbidden words from all hosts
    in their domain
  - **Obsolescence** — a concept becomes irrelevant (a mining town that
    runs out of ore forgets mining terms through lack of use)

- **Cultural contact causes borrowing.** When two cultures trade, their
  shared modules exchange vocabulary. The northern village learns
  `spice` from southern traders. The southern city learns `furs` from
  northern hunters. Borrowed words may be re-registered with modified
  meanings — the borrower's interpretation of the lender's concept.

- **Dialect divergence in parsing.** Over time, a region might develop
  different literal syntax. The `Dialect` trait's `ExprParser` is
  configurable (`packages/handl/crates/dialect/src/expr_parser/expr_parser.rs`).
  A region could use `yes`/`no` for booleans while another uses `true`/
  `false`. The same `Expr` is produced, but the surface form differs.
  The player must learn the local dialect.

- **Written language diverges too.** Inscriptions in ancient ruins use
  an older dialect — different literal syntax, different vocabulary. The
  player must learn to read the old form. This is a different `Lexer`
  implementation applied to the same world.

## Consequences

### Positive

- The world feels large — different regions genuinely speak differently.
- Language evolution is emergent — drift happens through module
  propagation and re-registration, not scripted events.
- Etymology and word death create deep gameplay — the player can trace
  history through language, lose words to cultural collapse, and fight
  censorship by re-teaching forbidden words.
- Ancient written language provides a discovery mechanic — deciphering
  old inscriptions is learning an old dialect.

### Negative

- Semantic drift can confuse players — the same word meaning different
  things in different regions is realistic but potentially frustrating.
  The meta-language (querying word meanings) mitigates this.
- Dialect divergence requires multiple `Dialect` and `Lexer`
  implementations — each regional variant is a separate configuration.
- Word death can soft-lock progression if a critical word is lost — the
  game must ensure recoverability (the player can always re-teach a word).

## References

- Game ADR-0001: Language — the protocol
- Game ADR-0004: Society — the graph
- `Dialect` trait: `packages/handl/crates/dialect/src/dialect/dialect.rs`
- `ExprParser` trait: `packages/handl/crates/dialect/src/expr_parser/expr_parser.rs`
- `LexerConfig`: `packages/handl/crates/lexer-canonical/src/config.rs`
- `ModuleHost`: `packages/handl/crates/runtime-core/src/host/host.rs`
