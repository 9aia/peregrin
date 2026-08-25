# Game Project Overview

## The one-sentence version

The player speaks a formal language into a world of formal minds, where every word is a function, every question is a suspended computation, every answer resumes a continuation, every transaction is an operation, every memory is a weighted log entry, every culture is a dialect of the same architecture, and every social bond is a shared module between two hosts.

## The architecture

Seven systems, one protocol:

| System     | Role                                                                | ADR                                    |
| ---------- | ------------------------------------------------------------------- | -------------------------------------- |
| Language   | The protocol — Expr trees, operations, continuations                | [0001](adrs/0001-language-protocol.md) |
| Psychology | The mind — attention, personality, emotion, activity                | [0002](adrs/0002-psychology-mind.md)   |
| Economy    | The material — items, prices, transactions, crafting                | [0003](adrs/0003-economy-material.md)  |
| Society    | The graph — shared modules, gossip, factions, relationships         | [0004](adrs/0004-society-graph.md)     |
| Culture    | The drift — vocabulary divergence, semantic drift, word death       | [0005](adrs/0005-culture-drift.md)     |
| Memory     | The continuity — weighted logs, personality-filtered storage, decay | [0006](adrs/0006-memory-continuity.md) |
| World      | The substrate — space, time, physics, inventory, environment        | [0007](adrs/0007-world-substrate.md)   |

## Foundation

Built on [handl](../../packages/handl/), a general-purpose expression language with a fully decoupled architecture. See:

## The core principle

**The language is the protocol. The host is the mind. The `Host` trait is the synapse.**

handl provides the language runtime (VM, lexer, parser, dialect, continuations). The game engine provides the minds (NpcMind, psychology, economy, society, culture, memory, world). They meet at the `Host` trait — a small, well-defined interface where every function lookup is a psychological decision and every operation signal is a speech act.
