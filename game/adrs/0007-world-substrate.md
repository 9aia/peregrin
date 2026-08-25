# ADR-0007: World — the substrate

## Status

Proposed

## Context

The language, psychology, economy, society, culture, and memory systems all
operate on top of a physical game world. There is space, time, physics,
inventory, and environmental state. These are not language concepts — they
are the substrate that the language acts upon.

The `Host` trait is the interface between the NPC's mind and the world.
When a handler resumes a continuation with `k(accept(item, payment))`, the
handler has already queried the world: checked inventory, verified the item
exists, confirmed the player can carry it. The world is not in the
language — it is what the language acts upon.

## Decision

The world is the substrate that all other systems query and modify through
the host:

- **Space.** The world has positions, regions, and proximity. The
  attention system (ADR-0002) queries space: is the speaker in hearing
  range? Is the NPC in their shop? The economy system (ADR-0003) queries
  space: is the item in this merchant's inventory at this location? The
  society system (ADR-0004) uses space for gossip propagation range.

- **Time.** The world has a clock — days, seasons, years. The memory
  system (ADR-0006) timestamps every entry. The culture system (ADR-0005)
  uses time for drift rate — words mutate over game-months, not
  real-time seconds. The economy system uses time for market dynamics —
  prices shift between day and night, season to season.

- **Physics.** The world has physical rules: items have weight, players
  have carrying capacity, doors can be locked, distances affect travel
  time. The host queries physics when evaluating operations: "can the
  player carry this item?" "is the door open?" "is the path blocked?"

- **Inventory.** Each NPC and the player have inventories. The economy
  system modifies inventories when trades complete. The crafting system
  consumes inventory items to produce new ones. Inventory is host state,
  updated by handlers, not by the language.

- **Environment.** Weather, lighting, terrain, and environmental hazards
  exist in the world. These affect the attention system (a storm reduces
  hearing range), the economy (rain increases fur prices), and the
  culture (a desert culture doesn't have words for snow).

- **The world is not in the language.** There is no `Expr` variant for
  "move north" or "pick up item." These are game engine operations
  triggered by the handler when an operation is signaled. The language
  says "give me the sword" — the handler queries the world, modifies
  inventory, and resumes the continuation. The world changed; the
  language only requested the change.

- **The world queries the host, not the VM.** The game engine does not
  evaluate `Expr` trees to simulate physics. The world simulation runs
  independently. The host is the bridge: when the world changes (a door
  opens, a storm begins), the game engine may update host state (register
  new functions, modify emotion, trigger memory storage). The language
  is one input to the world; the world is not an output of the language.

- **Written language is world state.** Signs, inscriptions, and scrolls
  are physical objects in the world. They contain serialized `Expr` trees
  in the conlang's written form. When a player reads them, the game
  engine feeds the serialized expression through the language pipeline.
  The inscription is world state; reading it is language evaluation;
  the effect (a door sealing, a spell being learned) is a world change
  triggered by the handler.

## Consequences

### Positive

- Clean separation: the world is the substrate, the language is the
  protocol, the host is the bridge. No system bleeds into another.
- The world simulation runs independently of the language — NPCs have
  lives, schedules, and activities whether or not the player is talking.
- Written language as physical objects creates a deep discovery mechanic
  — the world is full of readable expressions with real effects.
- Environmental state affects all systems coherently — a storm changes
  attention, prices, and available vocabulary in one coordinated update.

### Negative

- The world simulation is a large game engine system — space partitioning,
  time management, physics, inventory, and environment are all
  non-trivial to implement.
- The host must bridge world state and language dispatch — every handler
  that modifies the world must coordinate with the world simulation to
  avoid race conditions (two NPCs trying to pick up the same item).
- Written language requires a serialization format for `Expr` trees in
  the conlang's written form — this is a custom encoder/decoder on top
  of the existing AST.

## References

- Game ADR-0001: Language — the protocol
- Game ADR-0002: Psychology — the mind
- Game ADR-0003: Economy — the material
- Game ADR-0006: Memory — the continuity
- `Host` trait: `packages/handl/crates/vm/src/host.rs`
- `Continuation`: `packages/handl/crates/vm/src/continuation.rs`
- `Expr`: `packages/handl/crates/ast/` (handl-ast crate)
