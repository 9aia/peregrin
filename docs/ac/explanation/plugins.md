# Plugins (YAML-only composition)

There is no plugin system. You do not need one. **YAML files ARE plugins.**

A plugin is just a YAML fragment that someone includes. No code, no
registration, no package manager. Drop a file, include it, done.

## What a plugin looks like

A file in a shared directory:

```yaml
# plugins/notify/slack.yaml
id: notify-slack
driver: shell
on: [notify.requested]
emits: [notify.done]
with:
  cmd: 'curl -X POST -H ''Content-type: application/json'' --data ''{"text":"${event.payload.message}"}'' ${secret.SLACK_WEBHOOK_URL}'
```

A workflow "installs" it with one line:

```yaml
nodes:
  - include: plugins/notify/slack.yaml
```

## Why no plugin system?

A plugin system adds: registration, versioning, dependency resolution,
loading order, namespace management. You know what does not need any of that?

**Files.**

- **Install a plugin:** drop a YAML file in a directory.
- **Use a plugin:** `include: plugins/notify/slack.yaml`
- **Version a plugin:** git (the file is in a repo).
- **Compose plugins:** include multiple files.
- **Override a plugin:** deep-merge with local values.
- **Remove a plugin:** delete the include line.

The YAML IS the plugin. The include IS the installation. No code, no ceremony.

## See also

- [Build a plugin library](../how-to/build-a-plugin-library.md)
- [Compose from fragments](../how-to/compose-from-fragments.md) — `include:` and `${include()}`
