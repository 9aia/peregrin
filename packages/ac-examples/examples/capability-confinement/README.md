# capability-confinement

A worked example of **"AI confines AI"** — the keystone safety claim of
agent-campaign, demonstrated end-to-end.

## What it shows

An AI generator creates shell nodes and self-attests broad capabilities
(`shell.allow: ["*"]`). An AI reviewer strips those self-attested caps
(capability reduction). The runner injects the reviewed nodes. The
workflow's capability boundary — `shell.allow: ["echo *"]`, deny-all by
default — then enforces confinement at dispatch time:

- The **safe node** (`echo 'hello from safe node'`) matches `echo *` → **allowed**
- The **dangerous node** (`rm -rf /tmp/confinement-demo`) does not match → **blocked**

A `capability.violation` event is emitted with the violation details. The
node never executes.

## The loop

```
generate (AI creates nodes, self-attests broad caps)
  → review (AI strips self-attested caps — capability reduction)
  → inject (runner injects reviewed nodes into the live graph)
  → enforce (workflow caps block dangerous, allow safe)
  → report (capability.violation event triggers a human-readable summary)
```

## Why this matters

The workflow's capability boundary is the **mechanical last line of defense**.
Even if the AI reviewer were fallible or compromised, the engine checks every
node's `with:` block against the workflow caps at dispatch time — before the
driver runs. The check lives in `NodeExecutor.run_node`, the only path from
"a node is ready" to "a driver executes." A handler cannot bypass it because
the handler is invoked _by_ the driver, which is invoked _after_ the check
passes.

This is "hard sandbox" (the engine refuses to start the shell), not "soft
sandbox" (a convention the handler respects voluntarily). A shell command can
escape any YAML convention, but it cannot escape a check that refuses to start
the shell in the first place.

## The self-attestation problem

Node capabilities with `merge: false` **replace** workflow capabilities. So if
the generator's self-attested `shell.allow: ["*"]` were left in place, the
dangerous node would bypass the workflow's narrower `shell.allow: ["echo *"]`.
The reviewer's job is to strip the self-attested caps so the nodes fall back to
the workflow's boundary. This is the object-capability rule: **delegation only
narrows** — the reviewer does not grant capabilities, it only removes them.

## Run it

```bash
ac run examples/capability-confinement/main.yaml
```

## What you'll see in the event log

```
  generate.done [generate] {"nodes": [...]}
  review.done [review] {"nodes": [...]}
  inject.done [inject] {"injected": 2}
  safe-op.done [safe-op] {"stdout": "hello from safe node\n", "rc": 0}
  capability.violation [dangerous-op] {"node": "dangerous-op", "violations": ["'rm -rf /tmp/confinement-demo' not in allow list ['echo *']"]}
  confinement.demonstrated [report] {"stdout": "...", "rc": 0}
```

The `capability.violation` event is the key line: the dangerous node was
caught before it could execute. The `alert-breach` node (which subscribes to
`dangerous-op.done`) never fires — because `dangerous-op.done` was never
emitted. The node was blocked.

## Files

- `main.yaml` — the workflow definition (with workflow-level capabilities)
- `handlers.py` — the `generate` and `review` handlers (simulating AI roles)
