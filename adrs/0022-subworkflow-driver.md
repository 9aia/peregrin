# ADR-0022: Subworkflow driver

## Status

Proposed

## Context

The `include:` feature merges another YAML file into the current workflow. This is good for building large files from fragments, but it does not provide a callable boundary. The `lib/` node collection in `packages/ac-examples/examples/lib/` is presented as "data, not code" that users copy into `.ac/lib/`. Copying is not dependency management.

A reusable workflow should be callable with inputs and should return outputs, like a function. Today the only way to achieve isolation is to run a separate campaign and use `http` or `agent` to integrate it, or to use `include:` and accept the merged namespace.

## Decision

Introduce a `subworkflow` driver. The `subworkflow` driver runs another Peregrin YAML file in its own namespace, passes `inputs` as `config`, and maps the sub-workflow's final events back into the parent workflow.

```yaml
name: bugfix-campaign

nodes:
  - id: setup-worktree
    driver: subworkflow
    on: [start]
    emits: [worktree.ready, worktree.failed]
    params:
      path: lib/git/worktree.yaml
      inputs:
        repo: "${config.repo}"
        worktree_base: "${config.worktree_base}"
        branch_id: "${config.branch_id}"
      outputs:
        worktree.ready: worktree.ready
        worktree.failed: worktree.failed

  - id: run-fix
    driver: agent
    on: [worktree.ready]
    params:
      prompt_file: prompts/fix.md
      cwd: "${event.payload.worktree_path}"
```

The `subworkflow` driver:
1. Loads the target YAML.
2. Runs it with the provided `inputs` as its `config`.
3. Returns the first of the listed `outputs` events as the event for this node.
4. Forbids the sub-workflow from emitting into the parent namespace except through the mapped outputs.

## Consequences

- Reusable node libraries can be distributed as packages rather than copied files.
- Workflows can be tested in isolation and composed into larger campaigns.
- Input/output scoping makes workflows easier to reason about.
- A new package, `ac-driver-subworkflow`, is needed.
- The `ac-cli` may need a package/install command to fetch reusable workflow packages.

## Alternatives considered

- **Keep `include:` as the only composition primitive.** Rejected because merging does not isolate scope and makes it hard to track where a workflow came from.
- **Use `runner` injection to spawn a sub-graph.** Rejected because it requires the parent workflow to declare all sub-workflow nodes as data rather than referencing a file.
- **Use `agent` or `http` to invoke an external workflow.** Rejected because it introduces unnecessary process/network overhead for a purely local composition.
