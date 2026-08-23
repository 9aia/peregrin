# ADR-0003: Economy — the material

## Status

Proposed

## Context

The game world has physical resources: items, currency, inventory, trade.
These must interact with the language system — the player must be able to
request, buy, sell, craft, and exchange items through the conlang. The
economy must also be dynamic: prices respond to supply and demand, crafting
creates new items, and scarcity drives conflict.

The language layer provides operations (for trade requests) and functions
(for item references). The economy layer provides the logic that runs
inside the handler when a trade operation is signaled.

## Decision

The economy is modeled as functions and operations on the host:

- **Items are functions.** `glowstone()` is a registered function that
  returns a value representing the item. `glowstone.brightness()` returns
  a property. `glowstone.color()` returns another. Items compose: a
  `sword(material: iron(), edge: sharp())` is a function call that
  produces a composite item value.

- **Prices are values.** `price(50, coins())` evaluates to a price value.
  The economy system tracks market conditions and can produce different
  price values for the same item at different times. The price is not
  hardcoded — it is computed from supply, demand, and merchant personality.

- **Transactions are operations.** `"trade"(item, payment)` signals an
  operation. The NPC's handler intercepts it, checks inventory and price,
  and resumes the continuation with:
  - `k(accept(item, payment))` — trade succeeds, items move
  - `k(refuse("too cheap"))` — trade fails, NPC states why
  - `k(counter(price(75, coins())))` — NPC counter-offers

- **Crafting registers new functions.** When the player crafts a new item,
  the game engine registers a new function on the relevant hosts. The
  item did not exist in the language before. Now it does. The word is
  the function. The function is the item.

- **Market dynamics live in the handler.** The handler body (implemented
  by the game engine) queries the economy system:
  - Supply: how many of this item exist in the world?
  - Demand: how many players have requested this item recently?
  - Scarcity: is this item rare? Unique?
  - Relationship: does the merchant like this player?
  - Competition: are there other merchants selling the same item?

  These factors determine the resume value. The language doesn't know
  about supply and demand — the handler does.

- **Currency is a function.** `coins()` returns a currency value.
  `coins.amount()` returns the quantity. Different currencies are
  different functions: `gold()`, `silver()`, `barter_goods()`. A culture
  that doesn't use coins doesn't register `coins()` — they register
  `trade_goods()` instead. The economy adapts to the culture.

- **Inventory is host state.** Each NPC's `NpcMind` tracks their inventory.
  When a trade completes, the handler updates both inventories (NPC and
  player). The language triggered the change; the handler executed it.

## Consequences

### Positive

- Items are first-class words — they compose, they have properties, they
  can be invented.
- Prices are dynamic — computed from market conditions, not hardcoded.
- Crafting creates new vocabulary — the economy and language grow together.
- Different cultures can have different economic systems (currency vs
  barter) through different registered functions.

### Negative

- Every item type needs a function implementation on the host.
- Market dynamics require a simulation system behind the handler.
- Economic exploits must be guarded against in the handler logic (the
  language can't prevent a player from saying "give me everything for
  free" — the handler must refuse).

## References

- Game ADR-0001: Language — the protocol
- Game ADR-0002: Psychology — the mind
- `Host` trait: `packages/ac-lang/crates/vm/src/host.rs`
- `ModuleHost`: `packages/ac-lang/crates/runtime-core/src/host/host.rs`
