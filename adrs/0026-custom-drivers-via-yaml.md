# ADR-0026: Custom drivers via YAML

## Status

Proposed

## Context

The engine resolves `driver: <name>` on a node through `DriverRegistry`. Today the registry is populated in code by `ac-cli-defaults/src/index.ts`, which imports every built-in driver package and registers it. Adding a new project-specific driver means writing a package or changing the CLI.

Most "new" drivers are not new runtime families. They are configured instances of existing families: an HTTP endpoint, a shell profile, a CLI with a fixed working directory, or an agent with a specific system prompt. A team working with the Devin API, a GitHub bot, or an internal staging server wants a named driver without publishing `ac-driver-devin`.

The file router already discovers `.node.yml`, `.snippet.yml`, `.event.json`, `.handler.py`, `.config.yml`, and `.secret.yml` files. It can discover driver presets the same way. The template engine can already resolve `${...}` placeholders inside `params` values. This means a YAML file can define a driver that is a configured, named wrapper around an existing built-in.

## Decision

Add support for driver preset files discovered by `ac-file-router` and registered in `DriverRegistry`.

- Driver preset files use the extension `.driver.yml` or `.driver.yaml`. They may be placed in any directory; the extension is what matters, not a dedicated `drivers/` folder. Domain-driven folders are recommended (e.g. `integrations/devin.driver.yml`, `team-a/github.driver.yml`).
- A driver preset file has this shape:

  ```yaml
  # integrations/devin.driver.yml
  name: devin
  driver: http
  replayable: true
  params:
    method: POST
    url: https://api.devin.ai/v1/sessions
    headers:
      Authorization: "Bearer ${env.DEVIN_TOKEN}"
    body:
      prompt: "${params.task}"
  ```

- `name` is the public driver id (e.g. `driver: devin` on a node). It must be unique within a workflow.
- `driver` is the built-in or already-registered driver to delegate to (e.g. `http`, `shell`, `agent`, `cli`).
- `params` is the preset's default parameter map. It may contain `${...}` placeholders.
- `replayable` is optional and defaults to the base driver's own replayability. If `false`, any node that uses this preset is marked as non-replayable (see ADR-0027).
- The file router collects driver files and adds them to `DiscoveredFiles`.
- At startup, the engine loads each preset once and registers it with `DriverRegistry` under its `name`. The parsed `params` and base driver reference are cached; the final resolved parameter map is computed per node invocation because it depends on node `params`, events, and runtime context.
- The type generator must include discovered custom driver names in the `driver` enum so that schema validation and IDE completion know about them.
- When a node runs with `driver: <name>`:
  1. `DriverRegistry.resolve` checks built-ins first, then registered presets.
  2. If a preset is found, its `params` map is deep-merged with the node's `params`. The node-level values win.
  3. The merged map is resolved by `ac-template-engine` with the merged parameters exposed as `${params.*}`, workflow config as `${config.*}`, secrets as `${secret.*}`, and environment as `${env.*}`.
  4. The resolved map is passed to the base driver's `invoke`.
- A preset can declare `capabilities` and `resources` that are merged with the node's own declarations. Node declarations take precedence, but a preset can enforce a maximum safe set.

## Consequences

- Teams can create named, reusable drivers without a new package or CLI rebuild.
- Tokens, URLs, and other defaults move out of node files into preset files, where they can be versioned and secured separately.
- The `DriverRegistry` must support run-time registration from discovered files, not only compile-time registration in `ac-cli-defaults`.
- The `ac-file-router` needs a new suffix and a new list in `DiscoveredFiles`.
- Template resolution now happens in two places: once on the node `params` and once on the merged driver preset map. The engine must be careful not to double-resolve placeholders.
- There is a name-collision risk if a workflow names a preset `http` or `shell` and shadows a built-in. Built-ins take precedence; a warning is logged when a preset is ignored because of a name clash.
- The type generator must run after file discovery so custom drivers are available in the generated schema.

## Alternatives considered

- **Package-based plugin drivers.** Rejected. It is the right path for new driver families that need new runtime behavior, but it is too heavy for configured instances of existing drivers.
- **Register presets in the main workflow YAML.** Rejected. Driver presets are project-level assets that are reused across workflows; they belong in their own files.
- **Static `params` only, no template interpolation.** Rejected. Without placeholders, a preset could only hold constant defaults. The ability to map `${params.task}` into a JSON body is the main value.
