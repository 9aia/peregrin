+++
id = "ADR-0015"
title = "Replace `mode: all` with `dependsOn` glob patterns"
status = "open"
priority = "medium"
+++

# Replace `mode: all` with `dependsOn` glob patterns

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Context

Workflow nodes in `handl-core` currently carry a `mode` field with two values:

- `any` (default): the node fires on the first matching `on` pattern.
- `all`: the node waits until every `on` pattern has at least one matching event before it fires.

The `mode` enum is coarse. It forces every `on` pattern to act as both a trigger and a dependency, and it cannot express "wait for a set of events described by a glob". As workflows grow, authors need a more flexible fan-in: a node should be able to declare a family of events (for example `shard.*.done`) and trigger only once the entire family has been seen.

## Decision

1. Remove the `mode` field from the node model and from generated JSON Schemas.
2. Add a `dependsOn` field to each node. `dependsOn` is an array of glob strings.
3. The default trigger behavior is equivalent to the old `mode: any`: a node fires on the first `on` pattern that matches a published event. Because this is the default, `mode: any` is simply deleted; it has no explicit replacement.
4. When `dependsOn` is present, the node is only eligible to fire after every glob in `dependsOn` matches at least one already-seen event type. The actual trigger remains the first matching event on the `on` patterns.
5. `dependsOn` uses the same dot-segment glob dialect as `on`: `*` matches one segment, `**` matches one or more segments. It is evaluated by the existing `matches()` function in `handl-engine`, not by a separate library.
6. Schema generation, validation output, YAML examples, and documentation will be updated to use `dependsOn` instead of `mode: all`.

## Consequences

- Positive: clearer separation between what can trigger a node (`on`) and what a node waits for (`dependsOn`).
- Positive: glob patterns make fan-in over dynamic shards concise.
- Positive: the implicit default is the same as the old `mode: any`, so most nodes lose a field.
- Positive: `dependsOn` reuses the existing `on` pattern dialect, so there is no second glob syntax to learn and no new matching dependency.
- Negative: existing workflows that use `mode: all` must be migrated to `dependsOn: [...]`.
- Negative: existing workflows that explicitly set `mode: any` must have that field removed.
- Neutral: tests, examples, and documentation must be updated to reflect the new field.

## Status

proposed
