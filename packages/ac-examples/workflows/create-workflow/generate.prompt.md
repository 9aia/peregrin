# Generate an agent-campaign workflow from a spec.

You are generating a workflow for the **agent-campaign** framework — a
declarative event-graph orchestrator where workflows are pure YAML data.

## Inputs

- **Spec file**: `${SPEC_PATH}` — read this first. It describes what the
  campaign should do.
- **Output path**: `${WORKFLOW_PATH}` — where to write the main.yaml.
- **Workflow name**: `${WORKFLOW_NAME}` — the `name:` field in the YAML.

## Framework reference (read these before writing)

Read these files in the repo to understand the framework:

1. `README.md` — core concepts (Event, Node, Driver, Bus, Runner, Store)
2. `docs/guides/write-a-workflow.md` — node fields, branching, fan-in
3. `docs/guides/dynamic-fan-out.md` — wildcard subscriptions, map-reduce
4. `docs/guides/compose-from-fragments.md` — includes, snippets, lib nodes
5. `docs/guides/drivers.md` — shell, python, agent, http, runner drivers
6. `docs/guides/retry-loops.md` — max_fires, until, retire_on
7. `examples/bugfix-v2/` — complete workflow with agents, retry, fan-in
8. `examples/lib/` — reusable node YAMLs (git, test, agent, campaign)
9. `examples/map-reduce.yaml` — dynamic fan-out pattern
10. `agent_campaign/drivers/agent.py` — agent driver fields and behavior

## What to write

1. **`${WORKFLOW_PATH}`** — the main workflow YAML with:
   - `name`, `config`, `events`, `nodes`
   - Auto-discovered lib nodes (copy needed `.node.yml` files from
     `examples/lib/` into a `.ac/lib/` directory alongside the workflow)
   - Agent nodes with `prompt_file` references
   - Shell nodes for validate/verify/report steps

2. **Prompt files** — one `.prompt.md` per agent node, alongside the
   workflow YAML. Each prompt should include:
   - Goal (what the agent should do)
   - Prerequisites (what's already done)
   - Files to create/modify (with paths)
   - Constraints (what NOT to touch)
   - Test instructions

## Critical rules (these bit us before — follow them)

1. **Fan-in nodes (`mode: all`) MUST have a `driver:`** — a node without
   a driver doesn't execute and doesn't emit. Use `driver: shell` with a
   trivial `cmd: "echo 'wave complete'"`.

2. **Distinct event types per verify/check step** — if you reuse a lib
   node like `test.check` (which emits `test.check.passed`), calling it 3
   times causes event collisions. Either inline the check with distinct
   events (`verify1.passed`, `verify2.passed`) or use `max_fires: 0` with
   careful event routing.

3. **`prompt_file` paths are resolved relative to CWD** (the repo root
   when `ac run` is called from there), NOT relative to the workflow YAML.
   Use paths like `campaigns/my-workflow/phase1.prompt.md`.

4. **`forward:` replaces thin echo bridge nodes** — use
   `forward: [some.event]` instead of a shell node that just echoes.

5. **`set_state_from_stdout`** is a shell driver feature — it parses
   `KEY=VALUE` lines from stdout into `${state.KEY}`. The agent driver
   does NOT support this.

6. **`max_fires: 0`** means unlimited (fire per matching event), NOT
   "never fire". Omit `max_fires` for fire-once (default is 1).

7. **`mode: all`** waits for every `on` pattern to have at least one
   match before firing. `mode: any` (default) fires on first match.

8. **Parallel agents in the same wave must touch disjoint files** —
   agents run concurrently in the same worktree. If two agents edit the
   same file, they'll conflict.

9. **Every event in `on:` must be emitted by some node** — `ac validate`
   warns about orphan subscriptions. Check the warnings.

10. **`emit:` map overrides `emits:` list** — use `emit: {0: success,
default: failure}` for explicit exit-code-to-event mapping.

11. **Retry-exhaustion must route to an abort event** — when a node with
    `max_fires` exhausts its budget and another matching event arrives, the
    engine emits a `node.exhausted` event. Subscribe an abort/handler node
    to `node.exhausted` (checking `${event.payload.node}` if you need to
    distinguish which node exhausted) so the workflow terminates with a
    clear message. Without this, max_fires exhaustion silently quiesces to
    `workflow.done` — the run ends with neither "complete" nor "failed",
    which is ambiguous and hard to debug.

## Process

1. Read the spec file.
2. Read the framework docs and examples.
3. Plan the wave structure (which phases are parallel, which are sequential).
4. Write the main.yaml.
5. Write the prompt files.
6. Copy needed lib nodes to `.ac/lib/`.
7. Print a summary of what you created to stdout.

Do NOT run `ac validate` or `ac run` — downstream nodes will do that.
