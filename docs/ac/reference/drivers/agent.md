# agent driver

Runs an external AI coding agent CLI and maps exit codes to events.

## Basic usage

```yaml
- id: fix
  driver: agent
  on: [investigate.done]
  emits: [fix.done, fix.failed]
  with:
    runner: devin
    prompt_file: campaigns/my/fix.prompt.md
    permission_mode: dangerous
    cwd: "${state.worktree_path}"
    timeout: 600
    emit:
      0: fix.done
      default: fix.failed
```

## Runners

`devin`, `cline`, `claude-code`, `codex`, `custom`. Each maps to a known
invocation profile. `custom` requires `with.binary` and `with.flags`.

## Cross-agent fallback

Agent CLIs are opaque subprocesses. When rate-limited, the driver detects
this and backs off. If `with.runners` is an ordered list, it falls back to
the next runner:

```yaml
- id: fix
  driver: agent
  on: [investigate.done]
  emits: [fix.done, fix.failed]
  with:
    runners: [devin, cline]
    prompt_file: campaigns/my/fix.prompt.md
    permission_mode: dangerous
    rate_limit:
      max_retries: 2
      base_delay: 5
      max_delay: 120
      jitter: true
      cooldown: true
```

A process-global cooldown registry keeps concurrent agent nodes from
dogpiling a provider that is already throttling.

## Common `with:` fields

| Field             | Type   | Description               |
| ----------------- | ------ | ------------------------- |
| `runner`          | string | Agent CLI name            |
| `runners`         | list   | Ordered fallback list     |
| `prompt`          | string | Inline prompt             |
| `prompt_file`     | string | Path to prompt file       |
| `permission_mode` | string | Agent permission mode     |
| `trust`           | any    | Trust settings            |
| `cwd`             | string | Working directory         |
| `timeout`         | int    | Seconds                   |
| `binary`          | string | For `custom` runner       |
| `flags`           | list   | For `custom` runner       |
| `emit`            | dict   | Exit-code → event mapping |

## See also

- [Drivers](../../explanation/drivers.md) — the `agent:*` prefix convention
- [Safety](../../explanation/safety.md) — capability enforcement for agents
