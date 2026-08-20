# Getting started with Agent Campaigns

## Install

```bash
pip install -e .
```

## Your first workflow

Create `hello.yaml`:

```yaml
name: hello
nodes:
  - id: greet
    driver: shell
    on: [start]
    emits: [greet.done]
    params:
      cmd: "echo 'hello world'"
```

## Run it

```bash
ac run hello.yaml
```

Output:

```
============================================================
workflow complete: 2 events
============================================================
  start [__runner__]
  greet.done [greet] {"stdout": "hello world\n", "stderr": "", "rc": 0, ...}
  workflow.done [__runner__]
```

## Validate without running

```bash
ac validate hello.yaml
```

## Next steps

- [Write a workflow](write-a-workflow.md) — nodes, events, branching, fan-in
- [Run a workflow](run-a-workflow.md) — persistence, replay, observability
- [Compose from fragments](../how-to/compose-from-fragments.md) — includes anywhere
