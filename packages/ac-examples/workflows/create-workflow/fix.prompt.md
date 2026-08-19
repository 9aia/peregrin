# Fix issues in the generated workflow.

You are fixing issues found during validation, dry-run, or review of a
generated agent-campaign workflow.

## Inputs

- **Workflow path**: `${WORKFLOW_PATH}` — the YAML to fix
- **Spec path**: `${SPEC_PATH}` — the original spec (for context)
- **Review feedback**: the previous step's output (in the event payload
  that triggered this node — it contains the review or validation output)

## What to do

1. Read the current workflow YAML at `${WORKFLOW_PATH}`.
2. Read the review/validation feedback (it's in the event that triggered
   this fix — check the dry-run output and review output).
3. Re-read the framework docs if needed:
   - `docs/guides/write-a-workflow.md`
   - `docs/guides/drivers.md`
   - `docs/guides/retry-loops.md`
   - `examples/bugfix-v2/` (reference workflow)
4. Fix the issues identified in the review.
5. Write the updated YAML and any new/updated prompt files.

## Common fixes (from real experience)

1. **Fan-in node doesn't emit** → add `driver: shell` and
   `with: { cmd: "echo 'done'" }`.

2. **Event collision on repeated check** → inline the check with distinct
   event types (`verify1.passed` instead of `test.check.passed`).

3. **Prompt file not found** → fix the path to be relative to repo root
   (where `ac run` is called from), not relative to the workflow YAML.

4. **Orphan subscription** → either add a node that emits the missing
   event, or remove the subscription.

5. **Parallel agents editing same file** → restructure waves so agents
   in the same wave touch disjoint files.

6. **Missing abort path** → add abort/fix-exhausted nodes for failure
   events. If a node has `max_fires`, subscribe a handler to
   `node.exhausted` so exhaustion routes to an abort event instead of
   silently quiescing.

## Constraints

- Only fix the issues identified in the review. Don't refactor unrelated
  parts of the workflow.
- Keep the wave structure intact unless the review specifically calls it
  out.
- Print a summary of what you fixed to stdout.
