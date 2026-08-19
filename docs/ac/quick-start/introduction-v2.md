---
title: Introduction v2
---

# Introduction

Peregrin is a workflow engine for running long-lived automation that combines
scripts, APIs, AI agents, and human decisions. Workflows are declared as
event-driven graphs, persisted as an event log, and can react to failures, new
information, and new work.

A few terms used throughout the docs:

- **Workflow** — the executable graph declared in YAML.
- **Campaign** — a running instance of a workflow.
- **Event** — a message that carries data through the graph.
- **Node** — a handler attached to one or more event types.
- **Driver** — the adapter that performs the side effect when a node fires.

## A concrete example

Imagine a deploy pipeline. A pull request opens, tests run, a human has to
approve, an agent writes the release notes, and the change deploys.

```text
pr.opened → [ tests ] → build.done → [ human-approve ] → human.approved
                                             ↓
                                    human.rejected
                                             ↓
                                  [ notify-rejected ]

human.approved → [ release-notes ] → agent.notes → [ deploy ] → deploy.done
```

In Peregrin, each `[ box ]` is a node and each arrow is an event type. A node
fires when it has seen the events it is waiting for, runs a driver, and then
emits one or more new events. The engine walks the graph until nothing else is
ready to fire.

```yaml
workflow: deploy

events:
  - pr.opened
  - build.done
  - build.failed
  - build.timeout
  - human.approved
  - human.rejected
  - agent.notes
  - deploy.done

nodes:
  - id: tests
    driver: shell
    on: [pr.opened]
    emits: [build.done, build.failed, build.timeout]
    with:
      cmd: "npm test"

  - id: human-approve
    driver: human
    on: [build.done]
    emits: [human.approved, human.rejected]
    with:
      prompt: "Approve deploy for ${event.payload.branch}?"

  - id: release-notes
    driver: agent
    on: [human.approved]
    emits: [agent.notes]
    with:
      prompt: "Write release notes from ${events.pr.opened.payload}"

  - id: deploy
    driver: shell
    on: [agent.notes]
    emits: [deploy.done]
    with:
      cmd: "npm run deploy"
```

The `on` field declares the events that can trigger the node. The `emits` field
lists the events the node can produce. The `driver` and `with` fields say how
the side effect is performed. A node with `on: [a, b]` and `mode: all` waits for
both `a` and `b`; with `mode: any` (the default) it fires when the first
matching event arrives.

## The event-graph model

Peregrin is not a pipeline. It is a bipartite graph of event types and nodes.
Events connect nodes: a node consumes one or more events, performs a side effect,
and emits new events.

```text
pr.opened → [ tests ] → build.done
```

`pr.opened` and `build.done` are event types. `tests` is a node. The arrows show
that `tests` consumes `pr.opened` and produces `build.done`. A single node can
consume many events and emit many events; the graph is the set of all these
relationships.

This separation gives the engine a clear contract:

- A node does not know who produced the event it consumes. It only knows the
  event type. That makes nodes small, reusable, and easy to test in isolation.
- A driver cannot change the campaign directly. It can only return events, and
  the engine decides what to do with them.

```text
node + trigger events + context
            │
            ▼
         driver
            │
            ▼
       new events
```

Because every event is recorded, the engine can:

- **Reconstruct campaign state.** The engine can rebuild the state of a campaign
  from the event log without re-running external side effects.
- **Trace execution.** The log records which nodes fired, which events triggered
  them, and the resulting event sequence.
- **Recover from a crash.** The store is the source of truth for campaign state,
  so the engine can resume from the last persisted event.

## Drivers are the boundary for side effects

A driver is a small adapter that knows how to run one kind of handler. Built-in
drivers include:

- `shell` and `python` for local code.
- `http` and `webhook` for network calls.
- `agent` for launching an AI agent.
- `human` for interactive approval gates.
- `runner` for extending the workflow graph from inside a campaign.

Each driver receives a node, the events that triggered it, and an execution
context. It returns a list of events. This narrow contract makes it easy to add
drivers, mock them in tests, and reason about where the outside world is touched.

## Why YAML?

Peregrin workflows are written in YAML because the structure of the campaign
must be obvious and auditable. The file is the initial definition. The event log
is the record of what actually happened. Dynamic updates can change the live
graph, but they are themselves events that can be inspected and replayed.

YAML also makes it easy to generate, review, and version workflows alongside the
code they affect.

## Campaigns can grow as they run

Unlike a workflow whose graph is fixed before execution, a Peregrin campaign can
extend its graph while it runs. A `runner` node can:

- generate new workflow YAML,
- register new nodes in the current campaign,
- or persist those definitions as files for later reuse or for sharing as a
  library pattern.

This is useful when the campaign discovers sub-tasks as it runs, scaffolds a
review loop for a specific situation, or when an agent proposes the next steps
and the graph updates to include them.

The trade-off is that the campaign has more power, so the safety model matters.

## Runtime model

Peregrin is designed for production work. The runtime provides a few guarantees
about how campaigns execute:

- **Persistence.** Every event is written to a store before the next node is
  triggered. The store is the source of truth for campaign state.
- **Deterministic state transitions.** The engine evaluates the same event log
  the same way every time. Ordering within a single node is deterministic; nodes
  that are eligible at the same time may run concurrently unless constrained by
  resources or capabilities.
- **Crash recovery.** The engine can resume a campaign from the last persisted
  event without re-running side effects.

## Reliability and operations

Beyond the core runtime model, Peregrin provides features to keep campaigns
healthy in production:

- **Retries and circuit breakers.** Node-level retry policies and circuit
  breakers control how transient failures are handled.
- **Observability.** Observers receive every event, including OpenTelemetry
  traces and metrics.
- **Safety.** Capability checks constrain what a driver can do. A node's
  `capabilities` list declares what it may touch, such as `shell.exec`,
  `network.http`, `agent.spawn`, or `workflow.mutate`. The engine refuses to
  schedule a node that requests a capability the campaign has not granted.

## Built-in libraries and drivers

Peregrin ships with built-in drivers and a library of reusable workflow patterns
for common processes such as test-driven review, dependency upgrades, and
incident response. Users compose them with `include:`:

```yaml
include:
  - patterns/dependency-upgrade

nodes:
  - id: upgrade
    driver: runner
    on: [start]
    with:
      workflow: dependency-upgrade
```

## Who is Peregrin for?

Peregrin is for workflows that are too dynamic for a fixed CI pipeline but too
operationally important to remain an ad-hoc script:

- **Platform and SRE teams** wiring together infrastructure, CI, and incident
  response.
- **AI-product teams** running agent campaigns that loop, gather evidence, and
  ask for help.
- **DevOps teams** that want event-driven automation with a persistent audit
  trail and human-in-the-loop support.

## What to read next

- [Getting started](/ac/tutorials/getting-started) — install the CLI and run
  your first workflow.
- [Write a workflow](/ac/tutorials/write-a-workflow) — a deeper walk through
  the YAML model.
- [Drivers](/ac/explanation/drivers) — how drivers work and how to choose one.
- [Explanation](/ac/explanation/) — the concepts behind the engine.
