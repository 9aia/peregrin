# Peregrin

Peregrin is a workflow engine for building **event-driven, agent-aware campaigns**. It lets you describe multi-actor processes as YAML graphs of events, nodes, and drivers, then runs them with determinism, observability, and safety built in.

## What are Agent Campaigns?

Agent Campaigns is the workflow language and runtime on top of Peregrin. A campaign connects shell commands, Python handlers, HTTP calls, AI agents, human approvals, and dynamic graph injection in one declarative file. A node listens for events, invokes a driver, and emits new events. The engine walks the graph until nothing is ready to fire.

## Get started

- [Tutorials](ac/tutorials/) — learn by doing, step by step.
- [How-to guides](ac/how-to/) — achieve a specific goal.
- [Explanation](ac/explanation/) — understand why it works.
- [Reference](ac/reference/) — look up facts, schemas, and driver options.

This site is organised around the Diataxis documentation framework.
