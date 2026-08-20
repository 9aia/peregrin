---
title: Introduction v2
---

# Introduction

Most automation tools are either a fixed pipeline or an ad-hoc script.
Peregrin is an event-graph engine: you describe a workflow as a YAML graph of
event types and nodes, run it as a persistent campaign, and let the graph
extend itself when the work demands it.

## The core idea: events as edges, nodes as handlers

A workflow is a collection of nodes. Each node subscribes to one or more
**event types**, runs a handler through a **driver**, and **emits** new events.
Drivers are adapters such as `shell`, `python`, `http`, `webhook`, `agent`,
`human`, and `runner`; they receive the node and its trigger events, run the
side effect, and return new events. The engine walks the graph until nothing is
ready to fire.

A few terms:

- **Workflow** — the executable graph declared in YAML.
- **Campaign** — a running instance of a workflow; each campaign has its own
  append-only event log.
- **Event type** — a label such as `pr.opened` or `build.done` used in `on:` and
  `emits:`.
- **Event** — a concrete message with a payload and a source node id.
- **Node** — a handler attached to one or more event types.
- **Driver** — the adapter that performs the side effect when a node fires.

A node only knows the event type it consumes, not the node that produced it.
A driver cannot mutate the campaign directly; it can only return events, and the
engine decides what to do with them.

```text
node + trigger events + context
            │
            ▼
         driver
            │
            ▼
       new events
```

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

A campaign for this workflow starts when the engine receives an initial event
such as `pr.opened` — from a webhook, an API call, or the CLI. Each campaign
has its own event log; the same workflow can run many campaigns at the same
time.

In the YAML below, each `[ box ]` is a node and each arrow is an event type. A
node fires when it has seen the events it is waiting for, runs a driver, and
emits one or more new events.

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
    params:
      cmd: "npm test"

  - id: human-approve
    driver: human
    on: [build.done]
    emits: [human.approved, human.rejected]
    params:
      prompt: "Approve deploy for ${event.payload.branch}?"

  - id: release-notes
    driver: agent
    on: [human.approved]
    emits: [agent.notes]
    params:
      prompt: "Write release notes from ${events.pr.opened.payload}"

  - id: deploy
    driver: shell
    on: [agent.notes]
    emits: [deploy.done]
    params:
      cmd: "npm run deploy"
```

The `on` field declares the event types that can trigger the node. With
`mode: any` (the default) the node fires once for each matching event. With
`mode: all` it waits until every listed event type has at least one match in
the campaign log. Nodes can fire repeatedly as matching events arrive; use
`max_fires` to limit that. The `emits` field lists the event types the node can
produce.

In a `params` template, `event` refers to the trigger event. `events.<type>`
refers to an event of that type from the campaign log.

## Campaigns can grow as they run

Unlike a pipeline whose graph is fixed, a Peregrin campaign can extend its graph
while it runs. A `runner` node reads a list of node specs from its trigger
events and injects them into the live graph. This is how a campaign can plan
sub-tasks, scaffold a review loop, or absorb a generated workflow.

`runner` is a privileged driver: it is the only built-in driver that can change
the graph. A workflow that uses it should do so deliberately. Other drivers are
confined through capabilities; for example, a `shell` node can be limited to
`shell: { allow: ["git *"] }` so it cannot run arbitrary commands.

## Runtime model

The runtime gives a few guarantees:

- **Persistence.** Every event is written to a store before the next node is
  triggered. The store is the source of truth for campaign state.
- **Deterministic state transitions.** Given the same event log and workflow
  definition, the engine produces the same state transitions. Side effects are
  not replayed; only the logged events are.
- **Crash recovery.** The engine resumes a campaign by replaying the persisted
  log. A side effect can only repeat if the process crashed before the
  corresponding event was written.

## Why YAML

Peregrin uses YAML so the workflow structure is obvious and auditable. The file
is the initial definition; the event log is the record of what happened. YAML
also makes it easy to generate, review, and version workflows alongside the
code they affect.

## Built-in libraries and drivers

Peregrin ships with built-in drivers and a library of reusable workflow patterns
for processes such as test-driven review, dependency upgrades, and incident
response. Users compose them by adding an `include:` list to the workflow
definition:

```yaml
include:
  - patterns/dependency-upgrade

nodes:
  - id: upgrade
    driver: runner
    on: [start]
    params:
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
