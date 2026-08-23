# ADR-0002: Psychology — the mind

## Status

Proposed

## Context

Language provides the protocol, but an NPC needs a mind to process it. A
flat function registry (the default `ModuleHost`) produces identical NPCs
that know the same words and respond the same way. The game requires NPCs
that differ in attention, memory, personality, and emotional state.

The `Host` trait is the synapse between mind and language. Every
`Host::function` call is a psychological decision — not just "do I know
this word" but "am I paying attention, do I care, and would I respond?"

See Peregrin ADR-0035 for the full psychology system design.

## Decision

Each NPC has a `NpcMind` that implements `Host`. The mind has six
subsystems:

- **Attention** — filters whether the NPC notices a speaker at all. Runs
  before vocabulary lookup. Influenced by perception (line of sight),
  context (already in conversation?), activity (busy?), emotion (heightened
  or dulled awareness), and relationship (does the NPC care about this
  speaker?). If attention fails, `Host::function` returns `None` — the NPC
  does not react.

- **Personality** — stable traits that filter at three points: what the
  NPC notices (attention), what gets stored in memory (weighting), and how
  the NPC responds to the same operation (dispatch policy). A greedy
  merchant charges double. A generous friend gives freely. A suspicious
  guard refuses and questions. Same operation, different resume values.

- **Emotion** — transient state that modifies attention, memory weighting,
  and dispatch. Triggered by events (successful trade → satisfied, insult
  → angry, gift → grateful). Decays over time to a personality-determined
  baseline. Not simulated feeling — a priority modifier on dispatch.

- **Activity** — what the NPC is doing right now. Affects attention (a
  blacksmith hammering is less likely to notice casual speech) and context
  (a merchant at their stall expects trade talk). Scheduled by the game
  engine, not driven by language.

- **Vocabulary** — the `ModuleHost` from AC-Lang. The set of registered
  modules is the NPC's lexicon. Mutable at runtime — words are added
  through gameplay, removed through forgetting or censorship.

- **Memory** — see ADR-0006 (Memory system).

The `Host` implementation chains these filters:

```
function(name) → attention? → vocabulary? → personality+emotion? → return
```

Every `None` is a psychological decision. The VM cannot distinguish
"unknown word" from "not paying attention" from "chose to ignore." The
game engine tracks which filter caused the `None` for feedback.

### Lore framing

The game world is populated by beings whose minds are formal systems. The
machinic quality is the aesthetic, not a compromise. Attention is
protocol-governed perception. Personality is a dispatch policy. Emotion is
a resonance state. The player is the alien communicating with formal minds.

## Consequences

### Positive

- NPCs are differentiated — same utterance, different responses.
- Psychology is transparent — every decision is a dispatch result.
- The formal-minds lore turns limitations into aesthetic.

### Negative

- Six subsystems to build and tune.
- The `None` ambiguity requires engine-side diagnostic instrumentation.
- No emergent psychology — each subsystem is engine-driven.

## References

- Peregrin ADR-0035: Game psychology system for formal-minded NPCs
- `Host` trait: `packages/ac-lang/crates/vm/src/host.rs:36-89`
- `ModuleHost`: `packages/ac-lang/crates/runtime-core/src/host/host.rs`
