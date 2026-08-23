# ADR-0006: Memory — the continuity

## Status

Proposed

## Context

Without memory, every interaction is isolated. The player can say
something to an NPC, walk away, come back, and the NPC has no recollection.
The world has no continuity. Past actions don't affect future interactions.

The `Context` in AC-Lang provides per-evaluation variable scope, but it
does not persist across evaluations. Long-term memory must live in the
host — the `NpcMind` (see Game ADR-0002). The question is what gets
remembered, how it's stored, and how it's retrieved.

## Decision

Memory is a weighted, tagged, time-indexed log inside each `NpcMind`:

- **What gets stored is filtered by personality.** Not every event enters
  memory. The personality profile determines what passes the filter:
  - A **greedy** merchant stores transactions with high emotional weight,
    ignores gossip.
  - A **suspicious** guard stores anything involving weapons or strangers
    with high weight, ignores commerce.
  - A **curious** sage stores new words and concepts with high weight,
    ignores routine events.

  The personality filter runs at storage time — events that don't pass
  are never written. This is not human forgetting (where everything is
  stored and decays) — it is **selective logging** at write time.

- **Memory entries are structured:**

  ```rust
  struct Memory {
      event: MemoryEvent,
      emotional_weight: f32,
      player: PlayerId,
      timestamp: GameTime,
      tags: Vec<String>,
  }
  ```

  `emotional_weight` determines retrieval priority and decay rate.
  `tags` enable task-relevant retrieval. `timestamp` enables temporal
  queries ("what happened last week?").

- **Retrieval is task-relevant, not exhaustive.** An NPC does not scan
  all memories on every interaction. They retrieve memories tagged with
  the current context — the speaker, the topic, the location. This
  mirrors how formal minds work: log everything (that passes the filter),
  retrieve on demand.

- **Decay is log compaction.** Memories with low emotional weight that
  are never retrieved fade over time. This is not human forgetting — it
  is a retention policy. Old, irrelevant entries are pruned. The NPC's
  mind is a log-structured store with compaction, not a human brain.

  ```text
  Day 1:  Memory { event: "player said hello", weight: 0.1, tags: [greeting] }
  Day 50: Never retrieved → pruned by compaction policy

  Day 1:  Memory { event: "player bought sword", weight: 0.8, tags: [weapon, trade] }
  Day 50: Retrieved when player returns → retained, possibly weight adjusted
  ```

- **Memory affects dispatch.** When the player returns, the NPC's `Host`
  impl queries memory before responding:
  - Did this player wrong me? → relationship lowered, dispatch may refuse.
  - Did this player help me? → relationship raised, dispatch may offer
    discounts.
  - Did this player ask about this topic before? → NPC can reference
    prior conversation ("as I told you yesterday...").

- **Memory is queryable through the meta-language.** The player can ask
  "do you remember me?" or "what did I say last time?" — these are
  operations that query the `MemoryStore` and resume the continuation
  with the result.

- **Memory is not shared between NPCs.** Each `NpcMind` has its own
  `MemoryStore`. Information spreads between NPCs through the society
  system (shared modules — see Game ADR-0004), not through shared memory.
  An NPC who wasn't present for an event doesn't remember it — they
  learned about it through gossip.

- **Memory can be false.** Because information propagates through gossip
  (society system) with fidelity degradation, an NPC's memory may contain
  inaccurate information. They "remember" something that didn't happen
  — or happened differently than they think. This is not a bug — it is
  the consequence of information passing through personality filters at
  every hop.

## Consequences

### Positive

- The world has continuity — past actions affect future interactions.
- Memory is personality-dependent — different NPCs remember the same
  event differently.
- Memory decay prevents unbounded growth — log compaction keeps the store
  manageable.
- False memories emerge naturally from gossip fidelity degradation —
  creates realistic misunderstandings and conflict.
- Memory is queryable through the conlang — deepens the meta-language.

### Negative

- Memory store must be efficient — tag-indexed retrieval, not linear scan.
- Compaction policies need tuning — too aggressive loses important
  context, too lax grows unbounded.
- False memories can frustrate players — an NPC "remembering" something
  wrong must be handleable (the player can correct them, which creates a
  new memory entry).
- Memory is per-NPC — no shared memory pool. This is correct for the
  architecture but means each NPC's store is independent and must be
  individually managed.

## References

- Game ADR-0002: Psychology — the mind
- Game ADR-0004: Society — the graph
- `Context`: `packages/ac-lang/crates/vm/src/context.rs`
- `Host` trait: `packages/ac-lang/crates/vm/src/host.rs`
