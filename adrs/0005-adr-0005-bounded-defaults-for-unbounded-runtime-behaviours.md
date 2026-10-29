+++
id = "ADR-0005"
title = "ADR-0005: Bounded defaults for unbounded runtime behaviours"
status = "open"
priority = "medium"
+++

# ADR-0005: Bounded defaults for unbounded runtime behaviours

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Accepted

## Context

Several drivers and model defaults allow a node to run forever unless the workflow author explicitly adds a guard. This is convenient for quick experiments but unsafe for production:

- `handl-driver-human` defaults `timeout` to `0` (wait forever for stdin).
- `handl-driver-webhook` defaults `timeout` to `0` (wait forever for an HTTP POST).
- `handl-core` `Node.fromDict` sets `max_fires` to `0` (unlimited) for `poll` nodes when the field is missing.
- `handl-engine` `Bus.resolveFanInCount` returns `undefined` for a `fan_in` block with no `count` or `count_from`, which currently makes the node fire on the first matching event instead of waiting for an explicit number.
- A `poll` or `self-trigger` node with `max_fires: 0` and no `until` condition will run indefinitely.

Unbounded defaults cause real problems: CI jobs hang, agents run forever and rack up API costs, webhook servers block a runner, and recursive or self-triggering workflows consume the event bus. A typo or omission should degrade to a finite, safe limit rather than to an infinite one.

## Decision

All time, iteration, and fan-in defaults must be bounded. Authors can opt into unbounded behaviour only by writing an explicit `unlimited` or `0`. The engine and drivers treat missing or omitted limits as a sensible finite default.

### Concrete changes

1. **`max_fires` everywhere defaults to `1`**
   - Non-poll nodes already default to `1`.
   - `poll` nodes must not default to `0`. If the author does not specify `max_fires`, the node fires once and then retires.
   - To poll forever, the author must write `max_fires: unlimited` (after the `max_fires` type change to `'unlimited' | positive_int`) or a large explicit number.
   - `max_fires: 0` is no longer accepted; it is ambiguous and is rejected in `strict` mode.

2. **`timeout` defaults are finite per driver**

   | Driver    | Current default | Proposed default | `0` meaning         |
   | --------- | --------------- | ---------------- | ------------------- |
   | `human`   | `0`             | `300`            | explicit no timeout |
   | `webhook` | `0`             | `300`            | explicit no timeout |
   | `agent`   | `600`           | `600`            | unchanged           |
   | `shell`   | `30000 ms`      | `30000 ms`       | unchanged           |
   | `python`  | `30000 ms`      | `30000 ms`       | unchanged           |
   | `http`    | `300`           | `300`            | unchanged           |
   | `cli`     | `300`           | `300`            | unchanged           |
   | `polling` | `300`           | `300`            | unchanged           |

3. **`fan_in` requires a count**
   - A `fan_in` block without `count` or `count_from` is interpreted as `count: 1`.
   - In `strict` mode (ADR-0001) a missing count is a validation error.
   - `count` must be a positive integer; `0` or negative values are rejected.

4. **Poll and self-trigger guardrails**
   - `poll` nodes must declare an `until` condition, an explicit `max_fires`, or both.
   - `cli` and `polling` `self-trigger` mode defaults `max_fires` to `100` if omitted.
   - The runner emits `node.exhausted` when a node hits its `max_fires` limit.

5. **Workflow-level safety caps (future)**
   - Add an optional `max_runtime` (seconds) and `max_events` (event count) to `Workflow`. When either is exceeded, the runner emits a `workflow.exhausted` event and shuts down.
   - These are production knobs, not defaults, and are out of scope for the first pass.

### Examples after the change

```yaml
nodes:
  - id: ask_human
    driver: human
    on: [review.needed]
    emits: [review.approved, review.rejected]
    params:
      prompt: "Approve this change?"
      timeout: 0 # explicit: wait as long as necessary

  - id: watcher
    driver: webhook
    on: [start]
    emits: [webhook.received]
    params:
      port: 8080
      timeout: 0 # explicit: no timeout

  - id: poll_check
    driver: cli
    on: [start]
    emits: [check.done]
    params:
      cmd: "curl -sf https://api.example.com/health"
      interval: 5
      max_fires: 10 # bounded
```

## Consequences

- Workflows that relied on implicit unlimited polling or infinite human prompts will now stop early unless the author opts in.
- Documentation, examples, and tests that use `max_fires: 0` for 'unlimited' must be updated to `max_fires: unlimited` or a bounded number.
- The `handl-core` `Node.fromDict` default for `max_fires` on `poll` nodes changes from `0` to `1` (or the configured workflow default).
- `handl-driver-human` and `handl-driver-webhook` default `timeout` changes from `0` to `300`.
- `handl-engine` `Bus.resolveFanInCount` defaults a missing `fan_in` count to `1`.
- Agents and humans cannot accidentally hang a workflow; an explicit `timeout: 0` is a deliberate, visible choice.
- Production deployments become safer without requiring every node to be audited for implicit unbounded values.

## Alternatives considered

- **Keep unbounded defaults and add a global `strict` flag to enable bounded behaviour.** This was rejected because the safest behaviour should be the default; a `strict` flag is too easy to forget.
- **Use `0` to mean 'unbounded' but require it explicitly.** This is the current design and is exactly the source of the problem.
- **Add workflow-level `max_runtime` as the only guard.** This helps overall but does not make individual nodes safe by default.
