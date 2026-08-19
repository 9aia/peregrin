# Peregrin

Peregrin is a workflow engine for building **event-driven, agent-aware campaigns**. It lets you describe multi-actor processes as YAML graphs of events, nodes, and drivers, then runs them with determinism, observability, and safety built in.

## What are Agent Campaigns?

Agent Campaigns is the workflow language and runtime on top of Peregrin. A campaign connects shell commands, Python scripts, HTTP calls, AI agents, human approvals, and dynamic graph injection in one declarative file. A node listens for events, invokes a driver, and emits new events. The engine walks the graph until nothing is ready to fire.

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

## Get started

- [Tutorials](ac/tutorials/) — learn by doing, step by step.
- [How-to guides](ac/how-to/) — achieve a specific goal.
- [Explanation](ac/explanation/) — understand why it works.
- [Reference](ac/reference/) — look up facts, schemas, and driver options.

This site is organised around the Diataxis documentation framework.
