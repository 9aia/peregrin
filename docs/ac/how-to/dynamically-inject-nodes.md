# Dynamically inject nodes

There are two ways to inject nodes at runtime: through the `runner` driver or directly from a custom driver's `ctx.inject` call.

## Using the `runner` driver

A driver emits an event whose payload contains `nodes: [...]`. The `runner` driver picks up that event and injects the specs.

```yaml
nodes:
  - id: planner
    driver: planner
    on: [start]
    emits: [plan.ready]
    with:
      tasks:
        - cmd: "echo task-1"
        - cmd: "echo task-2"

  - id: absorb
    driver: runner
    on: [plan.ready]
    emits: [absorb.done]
```

The `planner` driver returns:

```ts
import { Effect } from "effect";
import { Driver, Event } from "ac-core";

const PlannerDriver: Driver = {
  name: "planner",
  invoke: (node, _triggers, _ctx) =>
    Effect.gen(function* () {
      const tasks = (node.with_.tasks ?? []) as Array<{ cmd: string }>;
      const newNodes = tasks.map((task, i) => ({
        id: `dynamic-${i}`,
        driver: "shell",
        on: ["absorb.done"],
        emits: [`dynamic-${i}.done`],
        with: { cmd: task.cmd },
      }));
      return [new Event("plan.ready", { nodes: newNodes }, node.id)];
    }),
};
```

## Using `ctx.inject`

A driver can also inject nodes directly through the `ExecutionContext` without emitting an intermediate `runner` event:

```ts
import { Effect } from "effect";
import { Driver, Event } from "ac-core";

const InjectorDriver: Driver = {
  name: "injector",
  invoke: (node, _triggers, ctx) =>
    Effect.gen(function* () {
      ctx.inject([
        {
          id: "dynamic-1",
          driver: "shell",
          on: ["start"],
          emits: ["dynamic-1.done"],
          with: { cmd: "echo hi" },
        },
      ]);
      return [new Event("injected", {}, node.id)];
    }),
};
```

## Scope injected nodes with `retire_on`

Injected nodes live until the workflow ends unless you remove them. Use `retire_on` to clean up:

```yaml
- id: temp-worker
  on: [task.assigned]
  emits: [task.done]
  retire_on: [task.done]
```

## See also

- [Self-modifying workflows](../explanation/self-modifying.md) — concept
- [Loop protection](../explanation/loop-protection.md) — `max_fires`, `until`, `retire_on`
- [runner driver reference](../reference/drivers/runner.md)
