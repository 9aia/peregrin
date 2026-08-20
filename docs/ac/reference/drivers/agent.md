# agent driver

Runs an external AI coding agent CLI and maps exit codes to events.

## Basic usage

```yaml
- id: fix
  driver: agent
  on: [investigate.done]
  emits: [fix.done, fix.failed]
  params:
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

The current TypeScript port supports:

- `devin` — runs `devin -p [--prompt-file <path> | <prompt>] [--permission-mode <mode>] --respect-workspace-trust <true|false>`
- `custom` — runs the binary and flags provided in `params.binary` and `params.flags`

## Common `params:` fields

| Field             | Type   | Description                                           |
| ----------------- | ------ | ----------------------------------------------------- |
| `runner`          | string | `devin` (default) or `custom`                         |
| `prompt`          | string | Inline prompt (mutually exclusive with `prompt_file`) |
| `prompt_file`     | string | Path to prompt file                                   |
| `permission_mode` | string | Agent permission mode (devin)                         |
| `trust`           | bool   | Workspace trust flag (devin, default `false`)         |
| `extra_args`      | list   | Extra CLI args appended to the invocation             |
| `env`             | dict   | Env vars for the agent process                        |
| `cwd`             | string | Working directory                                     |
| `timeout`         | int    | Seconds (default `600`)                               |
| `binary`          | string | For `custom` runner                                   |
| `flags`           | list   | For `custom` runner                                   |
| `emit`            | dict   | Exit-code → event mapping                             |

## Emitted payload

```yaml
stdout: "..."
stderr: "..."
rc: 0 | non-zero
runner: "devin" | "custom"
```

## See also

- [Drivers](../../explanation/drivers.md) — the `agent:*` prefix convention
- [Safety](../../explanation/safety.md) — capability enforcement for agents
