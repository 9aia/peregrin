+++
id = "ADR-0004"
title = "ADR-0004: Council driver for agent voting"
status = "open"
priority = "medium"
+++

# ADR-0004: Council driver for agent voting

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

Agent workflows often need to make controversial decisions where no single actor should decide alone. Examples include:

- Approving a risky deployment after reviewing code.
- Merging a hotfix that bypassed normal CI.
- Deciding whether a generated design is acceptable.

Today this can be modelled by hand with `fan_in` and a `tally` node, but the author has to wire:

- One voter node per agent or human.
- A `runner` or self-modifying node to spawn voters.
- A `fan_in` count that matches the number of voters.
- A `tally` node that reads all the votes and applies a decision rule.

This is powerful but verbose. A `council` driver would collapse the pattern into a single declarative node.

## Decision

Introduce a built-in `council` driver. The driver turns one node into a voting round: it collects votes from a configurable set of members, applies a decision rule, and emits the result.

### Example usage

```yaml
- id: review_pr
  driver: council
  on: [pr.opened]
  emits: [council.approved, council.rejected, council.tie, council.no_quorum]
  params:
    prompt: "Review this PR and vote yes or no. ${event.payload.diff}"
    members:
      - agent:claude
      - agent:cursor
      - human
    threshold: majority
    timeout: 600
    options: [yes, no]
    tie_breaker: human
```

### `params` fields

| Field            | Type   | Default                | Description                                                       |
| ---------------- | ------ | ---------------------- | ----------------------------------------------------------------- |
| `members`        | list   | required               | Voters; each can be `agent:<runner>`, `human`, or `driver:<name>` |
| `prompt`         | string | required               | Prompt given to every voter                                       |
| `prompt_file`    | string | none                   | Path to prompt file, alternative to `prompt`                      |
| `options`        | list   | `[yes, no]`            | Allowed vote values                                               |
| `threshold`      | string | `majority`             | `unanimous`, `majority`, `supermajority`, or `any`                |
| `supermajority`  | number | `0.66`                 | Fraction required when `threshold: supermajority`                 |
| `timeout`        | int    | `0` (no limit)         | Seconds to wait for all votes                                     |
| `tie_breaker`    | string | none                   | `human` or an `agent:<runner>` to break ties                      |
| `emit_ok`        | string | `emits[0]`             | Event emitted on a decision                                       |
| `emit_tie`       | string | `emits[1]` or fallback | Event emitted when tied                                           |
| `emit_no_quorum` | string | `emits[2]` or fallback | Event emitted when too few votes arrive                           |

### How it works

1. The driver resolves `params` values and the prompt.
2. For each member, it calls the corresponding voter:
   - `agent:<runner>` — invokes the `agent` driver with the same prompt.
   - `human` — uses the `human` driver to ask for input.
   - `driver:<name>` — uses an arbitrary registered driver.
3. Each voter returns one event with a `vote` payload field.
4. The driver tallies the votes and applies the threshold.
5. If a tie breaker is configured and the result is a tie, the tie breaker votes once and the final rule is applied again.
6. It emits the appropriate result event with a payload carrying:
   - `votes` — the collected vote events,
   - `counts` — a map of option → count,
   - `winner` — the winning option or `null`,
   - `quorum` — whether the configured members all voted.

### Implementation strategy

The `council` driver is **syntactic sugar over `fan_in` and existing drivers**. The engine does not need a new primitive. The driver can either:

- Run voters internally as sub-effects and collect their events, or
- Inject temporary voter nodes via the `runner`/`inject` mechanism and rely on `fan_in` to aggregate the votes.

For the first pass, the driver runs voters as sub-effects. This keeps the node atomic and avoids requiring `fan_in` wiring in the workflow file. Later, a `runner`-based implementation can enable true parallel execution and observability of each voter as a separate span.

## Consequences

- Controversial decisions become one node instead of a `runner` + N voters + `fan_in` + `tally` graph.
- The driver reuses the `agent` and `human` drivers, so existing prompt and permission conventions apply.
- A new package, `handl-driver-council`, is needed.
- The driver must handle partial votes (timeout, no quorum) and emit an explicit `no_quorum` event rather than hanging.
- The `council` driver is a good stress test for the `agent` and `human` drivers, for `fan_in` aggregation, and for the `strict` mode proposed in ADR-0001.
- Workflows that need custom voting rules (e.g. weighted votes, veto powers) can still fall back to the manual `fan_in` pattern.

## Alternatives considered

- **Only document the manual `fan_in` pattern.** This keeps the driver surface small, but councils are a common enough pattern that a dedicated driver is worth the sugar.
- **Add a `council` field to every node instead of a driver.** That would conflate the action being voted on with the voting process. A driver is cleaner.
- **Implement it purely as a `runner` injection.** That gives observability for each voter, but it requires `fan_in` wiring and is harder to implement in a single pass.
