# Peregrin Examples

A curated, categorized showcase of Peregrin workflows. These are intended to be read, validated, and adapted into real campaigns.

## Taxonomy

Examples are grouped by the skill they teach and the problem they solve, prefixed by progression:

| Prefix              | Focus                    | What it demonstrates                                           |
| ------------------- | ------------------------ | -------------------------------------------------------------- |
| `00-foundations`    | Core mechanics           | The event graph: nodes, events, fan-out, fan-in, branching.    |
| `10-patterns`       | Reusable patterns        | Resilience primitives you compose into bigger workflows.       |
| `20-real-world`     | End-to-end scenarios     | Concrete, multi-actor automation problems.                     |
| `30-agentic-safety` | Agents and capabilities  | Human-in-the-loop, agent councils, and capability confinement. |
| `40-meta`           | Self-modifying workflows | Workflows that generate, validate, or extend other workflows.  |

Every example file starts with a header comment explaining:

- The problem it solves.
- The key Peregrin features it uses.
- How to validate it safely (`validate` is recommended).
- Any required secrets or environment.

## Running an example

Validate a workflow from the repo root:

```bash
node apps/cli/dist/cli.mjs validate examples/00-foundations/00-hello.yaml
```

To run for real:

```bash
node apps/cli/dist/cli.mjs run examples/00-foundations/00-hello.yaml
```

Some examples call external services or prompt a human; read the header comment first.

## Tag index

| Example                      | Drivers                     | Patterns                    | Concepts                            |
| ---------------------------- | --------------------------- | --------------------------- | ----------------------------------- |
| `00-hello`                   | shell                       | —                           | start, emits                        |
| `01-parallel-checks`         | shell                       | fan-out, fan-in             | mode: all, conditional branching    |
| `02-branching`               | shell                       | branching                   | on, mode: any                       |
| `10-bounded-retry`           | shell                       | retry, bounded loop         | max_fires, until, emits mapping     |
| `11-circuit-breaker`         | http, shell                 | circuit breaker             | on_trip, fallback                   |
| `12-polling`                 | shell                       | polling                     | poll:, until                        |
| `13-saga`                    | shell                       | saga, compensation          | failure events, compensation        |
| `20-deploy-pipeline`         | shell, human                | fan-out, fan-in, human gate | multi-actor, rollback               |
| `21-pr-review-bot`           | http, shell, agent          | fan-out, conditional        | GitHub-style review flow            |
| `22-incident-response`       | webhook, http, shell, human | escalation, paging          | severity routing                    |
| `30-human-gate`              | human, shell                | human-in-the-loop           | approval, timeout                   |
| `31-capability-confinement`  | python, shell, runner       | capability safety           | generated nodes, policy enforcement |
| `32-agent-council`           | shell                       | fan-out, fan-in             | simulated multi-agent decision      |
| `40-self-modifying-scaffold` | python, runner              | meta-workflow               | generate + inject nodes             |

## Notes

- Examples in `30-agentic-safety` and `40-meta` that use `runner` or `agent` drivers assume a planner handler or agent CLI is configured. They are designed as patterns to wire in once the handler side is ready.
- All shell commands are safe and idempotent; they use `echo`, read-only queries, or dry-run-safe placeholders.
- The examples are written against the Peregrin YAML conventions shown in `docs/ac/`. If a driver in the current TypeScript implementation differs, the YAML itself is the intended reference pattern.
