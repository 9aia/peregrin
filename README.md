# Peregrin

Peregrin is a workflow engine for building **event-driven, agent-aware campaigns**. It lets you describe multi-actor processes as YAML graphs of events, nodes, and drivers, then runs them with determinism, observability, and safety built in.

Agent Campaigns is the workflow language and runtime on top of Peregrin. A campaign connects shell commands, Python handlers, HTTP calls, AI agents, human approvals, and dynamic graph injection in one declarative file. A node listens for events, invokes a driver, and emits new events. The engine walks the graph until nothing is ready to fire.

## Development

- Check everything is ready:

```bash
vp run ready
```

- Run the tests:

```bash
vp run -r test
```

- Build the monorepo:

```bash
vp run -r build
```

- Run the development server:

```bash
vp run dev
```

## Documentation

The full documentation lives in [`docs/`](docs/) and is published as a VitePress site.
