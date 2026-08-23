# ADR-0004: Society — the graph

## Status

Proposed

## Context

NPCs are not isolated minds. They exist in a social graph — they have
relationships, share information, form factions, and influence each other.
When the player interacts with one NPC, the consequences should ripple
through the social network. A merchant who was wronged should tell other
merchants. A guard who heard something suspicious should alert other guards.

The `ModuleHost` architecture provides the mechanism: hosts can share
modules. If two NPCs share a `social` module, state changes in one are
visible to the other. This is not a separate system bolted on — it is a
consequence of host composition.

## Decision

Society is modeled as shared module state between hosts:

- **Shared modules are the social fabric.** Two NPCs in the same guild
  share a `guild` module. Two guards in the same garrison share a
  `garrison` module. State written to a shared module by one NPC is
  visible to all NPCs who share it. This is the mechanism for gossip,
  reputation, and coordinated behavior.

- **Gossip is module state synchronization.** When a merchant learns
  something (a player bought a weapon), the `social` module's state
  updates. The game engine syncs this state to other NPCs who share the
  module, possibly with delay (gossip travels at the speed of the sync
  rate, which can be tied to in-game time and proximity).

- **Relationships are per-player dispatch policies.** Each NPC's `NpcMind`
  tracks a `Relationship` per player. The relationship modifies the
  `Host` dispatch: a high-relationship player gets better prices, more
  attention, and access to private vocabulary. A low-relationship player
  gets refused, ignored, or reported.

- **Factions are module groups.** A faction is a set of NPCs who share a
  common module. The faction module tracks faction-level state: goals,
  territory, reputation with other factions, membership roster. Joining a
  faction means registering the faction module on the NPC's host. Being
  expelled means deregistering it.

- **Social rank affects dispatch priority.** Within a faction, rank
  determines which operations an NPC will accept from whom. A low-rank
  guard won't accept a `"command"` operation from a civilian. A
  high-rank commander will. This is personality + relationship + faction
  rank combined in the dispatch filter.

- **Information has propagation rules.** Not all shared state syncs
  immediately. The game engine controls:
  - **Speed** — gossip travels over hours/days, not instantly
  - **Fidelity** — information degrades as it propagates (details lost,
    emotional weight amplified or dampened by each NPC's personality)
  - **Range** — some modules are local (same village), some are regional
    (same guild), some are global (same religion)

- **The player is a node in the graph.** The player has their own host.
  When an NPC teaches the player a word, the game engine registers it on
  the player's host. When the player shares information, it propagates
  through the player's social connections. The player is not outside the
  social system — they are a participant.

## Consequences

### Positive

- Social consequences emerge from host composition, not scripted events.
- Gossip, reputation, and faction dynamics are all module state sync.
- The player is embedded in the social graph, not separate from it.
- Information propagation can be tuned (speed, fidelity, range) for
  gameplay pacing.

### Negative

- Module sync requires a game engine system to manage propagation
  timing, fidelity degradation, and range limits.
- Shared mutable state between hosts introduces consistency challenges —
  what if two NPCs update the same guild module simultaneously?
- The social graph can produce unintended consequences (a rumor
  spreading to the wrong NPC at the wrong time) that must be handled
  gracefully.

## References

- Game ADR-0002: Psychology — the mind
- Game ADR-0006: Memory — the continuity
- `ModuleHost`: `packages/ac-lang/crates/runtime-core/src/host/host.rs`
- `Host` trait: `packages/ac-lang/crates/vm/src/host.rs`
