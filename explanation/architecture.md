# Architecture

Mechanism vs policy: [sepia](https://github.com/9aia/sepia) is the
mechanism — it does things *to* sessions. Peregrin is the policy — it
decides *who/when/where* (arbitration, routing, scheduling, offline
ordering).

`peregrin-node` spawns `sepia serve` and drives it over ndjson JSON-RPC
stdio — no linked sepia internals, so sepia upgrades ship independently.
