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
- **Event** — a message that moves execution through the graph.
- **Node** — a handler attached to one or more event types.
- **Driver** — the adapter that performs the side effect when a node fires.

## A concrete example

Imagine a deploy pipeline. A pull request opens, tests run, a human has to
approve, an agent writes the release notes, and the change deploys.

```text
   pr.opened
      │
      ▼
  ┌─────────┐
  │  tests  │
  └────┬────┘
       │
       ▼
   build.done
       │
       ▼
  ┌────────────┐
  │   human    │
  │  approve   │
  └─────┬──────┘
        │
        ▼
  human.approved
        │
        ▼
  ┌──────────┐
  │  agent   │
  │  notes   │
  └─────┬────┘
        │
        ▼
   deploy
```

In Peregrin, each box is a node and each arrow is an event type. A node fires
when it has seen the events it is waiting for, runs a driver, and then emits one
or more new events. The engine walks the graph until nothing else is ready to
fire.

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

The `on` field says what the node waits for. The `emits` field says what it can
produce. The `driver` and `with` fields say how the side effect is performed. The
campaign starts at `pr.opened`, fires `tests`, then moves through the graph based
on the events that come back.

## The event-graph model

Peregrin is not a pipeline. It is a bipartite graph of events and nodes.

- **Events are the edges.** They connect nodes, carry data, and are written to a
  persistent log.
- **Nodes are the handlers.** They wait for events, run a driver, and emit more
  events.
- **Drivers are the boundary with the outside world.** They are the only place
  where side effects are performed.

This separation gives the engine a clear contract. A node cannot know who
produced the event it consumes. It only knows the event type. That makes nodes
small, reusable, and easy to test in isolation. A driver cannot change the
campaign directly. It can only return events, and the engine decides what to do
with them.

Because every event is recorded, the engine can:

- **Replay** the campaign from the log.
- **Recover** from a crash by reading the latest state.
- **Trace** exactly which nodes fired and in what order.

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

## Campaigns can grow as they run

A workflow does not have to be fixed before it starts. A `runner` node can
inspect, extend, and write new workflow YAML files and then inject fresh nodes
into the live campaign. This is useful when:

- The campaign discovers sub-tasks as it runs.
- The campaign needs to scaffold a review loop for a specific situation.
- An agent proposes the next steps and the graph updates to include them.

This makes Peregrin different from most workflow engines, where the graph is
frozen before the first step. The trade-off is that the campaign has more power,
so the safety model matters.

## Runtime guarantees

Peregrin is designed for production work. The runtime provides a few guarantees
to keep campaigns reliable:

- **Persistence.** Every event is written to a store before the next node is
  triggered. The store is the source of truth for campaign state.
- **Replay.** The engine can re-evaluate a campaign from the event log. This is
  not the same as re-running side effects; it is a deterministic replay of the
  state transitions.
- **Retries and circuit breakers.** Node-level retry policies and circuit
  breakers control how transient failures are handled.
- **Observability.** Observers receive every event, including OpenTelemetry
  traces and metrics.
- **Safety.** Capability checks constrain what a driver can do, so a node cannot
  silently exceed the permissions given to the campaign.

## Why YAML?

Peregrin workflows are written in YAML because the structure of the campaign
must be obvious and auditable. The file is the initial definition. The event log
is the record of what actually happened. Dynamic updates can change the live
graph, but they are themselves events that can be inspected and replayed.

YAML also makes it easy to generate, review, and version workflows alongside the
code they affect.

## Built-in libraries and drivers

Peregrin ships with built-in drivers and a library of reusable workflow patterns
for common processes such as test-driven review, dependency upgrades, and
incident response. Users compose them with `include:` or install them from a
library, so teams do not have to reinvent patterns that are already tested.

## Who is Peregrin for?

Peregrin is for workflows that are too dynamic for a fixed CI pipeline but too
operationally important to remain an ad-hoc script:

- **Platform and SRE teams** wiring together infrastructure, CI, and incident
  response.
- **AI-product teams** running agent campaigns that loop, gather evidence, and
  ask for help.
- **DevOps teams** that want event-driven automation with a persistent audit
  trail and human-in-the-loop support.

If your automation is starting to feel like a fragile pile of scripts and
webhooks, Peregrin is worth a look.

## What to read next

- [Getting started](/ac/tutorials/getting-started) — install the CLI and run
  your first workflow.
- [Write a workflow](/ac/tutorials/write-a-workflow) — a deeper walk through
  the YAML model.
- [Drivers](/ac/explanation/drivers) — how drivers work and how to choose one.
- [Explanation](/ac/explanation/) — the concepts behind the engine.
