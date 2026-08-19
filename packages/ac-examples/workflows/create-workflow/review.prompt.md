# Review the generated workflow.

You are reviewing a workflow generated for the **agent-campaign** framework.
Your job is to check the dry-run output and the workflow YAML for
correctness, then write PASS or FAIL.

## What to review

1. **Read the workflow YAML**: `${WORKFLOW_PATH}`

2. **Read the dry-run output**: it was captured from the previous step.
   The dry-run output is in the `DRYRUN_OUTPUT` env var.

3. **Run both checks yourself** to see the full output:
   ```bash
   cd "${config.repo}"
   ac validate "${WORKFLOW_PATH}" 2>&1
   ac run "${WORKFLOW_PATH}" --dry-run --no-store 2>&1
   ```

## Checklist (these are real issues we've seen)

### Structural (ac validate should catch these)

- [ ] Every event in a node's `on:` is emitted by some other node
- [ ] No orphan subscriptions (validate warns about these)
- [ ] `emits:` lists match `emit:` maps (no contradictions)

### Runtime (dry-run should catch these)

- [ ] All nodes fire (check the INFO log lines)
- [ ] No DLQ entries (node failures)
- [ ] No `workflow.failed` event
- [ ] Event flow matches the intended pipeline
- [ ] `prompt_file` paths resolve (no "cannot read prompt_file" errors)

### Semantic (only agent review catches these)

- [ ] **Fan-in nodes (`mode: all`) have a `driver:`** — without a driver,
      the node doesn't execute and doesn't emit. This is the #1 gotcha.
- [ ] **Distinct event types per repeated step** — if a check/verify step
      runs multiple times, each instance must use distinct event types
      (e.g. `verify1.passed`, `verify2.passed`), not shared events.
- [ ] **Parallel agents touch disjoint files** — no two concurrent agents
      in the same wave edit the same file.
- [ ] **`max_fires` is set on retry nodes** — the fix/iterate node should
      have `max_fires` to bound the loop.
- [ ] **`forward:` is used instead of echo bridge nodes** — no shell nodes
      that just `echo` to bridge events.
- [ ] **`prompt_file` paths are relative to repo root** (where `ac run`
      is called from), not relative to the workflow YAML.
- [ ] **Prompt files exist and are non-empty**.
- [ ] **Lib nodes are in `.ac/lib/`** if used (auto-discovered by the
      file router).
- [ ] **`set_state_from_stdout` is only used on shell nodes** (not agent
      nodes — the agent driver doesn't support it).
- [ ] **State variables are set before they're read** — e.g.
      `${state.worktree_path}` must be set by a git.worktree node before
      downstream nodes reference it.
- [ ] **`node.exhausted` is handled** — every node with `max_fires` > 1
      has a downstream node subscribed to `node.exhausted` that routes to
      an abort/complete event. Without this, max_fires exhaustion silently
      quiesces to `workflow.done` (ambiguous — neither success nor failure).

### Best practices

- [ ] Events are declared in the `events:` section
- [ ] Config values are used for repo paths, commands, etc.
- [ ] Abort/fix-exhausted nodes handle failure paths
- [ ] Report node prints a completion summary
- [ ] Prompt files include constraints (what NOT to touch)
- [ ] Prompt files include test instructions

## Output

Write **PASS** or **FAIL** to stdout, followed by your review.

If FAIL, list the specific issues that need fixing, with file paths and
line references. The fix agent will use your review as input.
