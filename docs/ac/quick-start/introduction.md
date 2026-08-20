---
title: Introduction
---

# Introduction

Peregrin is a workflow engine for building **event-driven, agent-aware campaigns**. It lets you describe processes as YAML graphs of events, nodes, and drivers, then runs them with determinism, observability, and safety built in.

If you are wiring together shell scripts, AI agents, HTTP calls, human approvals, and retries, Peregrin gives you a single model for declaring how those pieces fit together and a runtime that keeps the state machine honest.

## What problem Peregrin solves

Modern automation is rarely a single command. It is a chain of decisions, side effects, failures, and handoffs:

- A pull request opens → run tests → ask a human → call an agent → deploy.
- A ticket is created → classify → spawn sub-tasks → wait for evidence → close.
- A periodic check detects drift → gather data → open a campaign → loop until resolved.

Each step is a different actor: a shell, a Python script, an HTTP API, a coding agent, a human. Each actor can fail, retry, time out, or produce new work. Existing tools tend to treat these as either a static pipeline or an ad-hoc script. Peregrin treats them as a graph that can grow, react, and repair itself.

## The core idea: events as edges, nodes as handlers

Peregrin is built around a small, event-graph model.

A **workflow** is a collection of **nodes**. Each node listens for one or more **events** and, when fired, invokes a **driver**. The driver runs a handler and **emits** new events. Those events trigger more nodes. The engine walks the graph until nothing is ready to fire.

This sounds simple, but the shape gives you several powerful properties:

- **Event types are the interface.** A node does not know who produced the event it consumes, only the event type. This lets you compose small, reusable handlers.
- **Graphs can self-modify.** A `runner` node can inject new nodes into the live workflow, so the campaign can plan its next steps based on what it has learned.
- **Everything is observable.** The engine records every event in a store, so you can replay, debug, or analyse the exact sequence that occurred.
- **Failure is explicit.** A node emits specific events for success, timeout, or failure. The graph, not the engine, decides what happens next.

## Why YAML?

Peregrin workflows are written in YAML because the goal is to make the structure of a campaign obvious and auditable. The file is the source of truth for what can happen and under what conditions.

A minimal workflow looks like this:

```yaml
workflow: greet

events:
  - start

nodes:
  - id: hello
    driver: shell
    on: [start]
    emits: [greet.done]
    params:
      cmd: "echo 'Hello, ${event.payload.name}'"
```

The `driver` decides how the handler runs. The `on` field declares the trigger. The `emits` field lists the possible next events. The `params` object carries the driver-specific payload and supports `${...}` placeholders.

YAML also makes it easy to generate, review, and version workflows alongside the code they affect.

## Drivers: the contract between the engine and the world

A **driver** is a small adapter that knows how to run one kind of handler. Built-in drivers include:

- `shell` and `python` for local code.
- `http` and `webhook` for network calls.
- `agent` for launching external coding agents.
- `human` for interactive approval gates.
- `runner` for dynamically extending the workflow graph.

Drivers are the only place where side effects happen. They receive the node, the triggering events, and a context, then return a list of events. This narrow contract makes it easy to add new drivers, mock them in tests, and reason about where the outside world is touched.

## Safety, resilience, and observability

Peregrin is meant for production work, so it is designed to be observable and safe by default:

- **Stores** persist the event log, allowing replay and state recovery.
- **Observers** expose traces and metrics, including OpenTelemetry support.
- **Resilience** patterns — retry, circuit breaker, dead-letter queues — are modelled as node properties and runtime policies.
- **Safety** capabilities constrain what a driver can do, so a workflow cannot accidentally escalate beyond its permissions.

These are not afterthoughts. They are part of the model, and they appear in the YAML alongside the business logic.

## Meta-workflows

A workflow does not have to stop at running commands. Nodes can inspect, extend,
and write new workflow YAML files. A meta-workflow can scaffold a CI workflow,
draft a human-review loop, or generate a new campaign file and commit it for
review — either inside the same repository or to a shared AC library. The engine
treats workflow files as just another artifact that campaigns can produce.

## Built-in libraries and drivers

Agent Campaigns ships with built-in libraries of validated, reusable YAML
patterns — common processes like test-driven review, dependency upgrades,
incident response, and agent councils — plus built-in drivers for shell, Python
scripts, HTTP, webhooks, agents, humans, and more. Users compose them with
`include:` or install them from a library, so teams do not have to reinvent
patterns that have already been tested, validated, and evaluated.

## Effect and TypeScript

The current TypeScript port of Peregrin is implemented with [Effect](https://effect.website/). Effect gives the engine a coherent story for errors, concurrency, resource management, and tracing. It also makes the codebase easier to test and compose.

You do not need to know Effect to write workflows, but it underpins the runtime that runs them.

## Who is Peregrin for?

Peregrin is designed for teams that need to automate complex, multi-actor processes:

- **Platform and SRE teams** wiring together infrastructure, CI, and incident response.
- **AI-product teams** running agent campaigns that loop, gather evidence, and ask for help.
- **DevOps teams** that want event-driven automation with a persistent audit trail and human-in-the-loop support.

If your automation is starting to feel like a fragile pile of scripts and webhooks, Peregrin is worth a look.

## What to read next

- [Getting started](/ac/tutorials/getting-started) — install the CLI and run your first workflow.
- [Write a workflow](/ac/tutorials/write-a-workflow) — a deeper walk through the YAML model.
- [Drivers](/ac/explanation/drivers) — how drivers work and how to choose one.
- [Explanation](/ac/explanation/) — the concepts behind the engine.
