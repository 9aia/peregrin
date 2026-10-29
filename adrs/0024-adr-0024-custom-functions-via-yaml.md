+++
id = "ADR-0024"
title = "ADR-0024: Custom functions via YAML"
status = "open"
priority = "medium"
+++

# ADR-0024: Custom functions via YAML

## Status

Superseded by [handl ADR-0019](../../packages/handl/project/adrs/0019-custom-functions.md). Custom functions are now a language extensibility feature.

Proposed

## Context

Built-in template functions are fast and reliable, but adding one requires a code change to `handl-template-engine`. The community should be able to define domain-specific helpers without publishing a new package or getting a change merged into the engine. Some helpers are also too project-specific to deserve a built-in: an internal URL shortener, a company-specific ID generator, or a wrapper around a proprietary API.

The workflow already has a driver/preset model where `drivers/*.yml` wraps a base driver with default `params` values. Functions are a natural extension of this idea: a function is a small, reusable expression that can be invoked from a template, and its behavior can be implemented by a driver.

## Decision

Support user-defined functions declared in `*.fn.yml` files.

- A function file follows this shape:

  ```yaml
  # helpers/short-id.fn.yml
  name: short-id
  params:
    - name: prefix
      default: "id"
  driver: shell
  params:
    cmd: "echo '${params.prefix}-$(date +%s)-${random(1000, 9999)}'"
  ```

- The file extension is `.fn.yml` or `.fn.yaml`.
- Function files may be placed in any directory; the extension is what matters, not a dedicated `functions/` folder. Domain-driven folders are recommended (e.g. `helpers/short-id.fn.yml`, `sales/currency.fn.yml`) but optional.
- `params` declares the parameters the function accepts; `params` is the merged map that the base driver sees.
- Inside `params`, the function uses `${params.<name>}` to access its arguments and any built-in function to compute the result.
- The template engine resolves `${short-id("task-")}` by:
  1. Discovering `short-id.fn.yml` anywhere in the project.
  2. Binding the call arguments to `params`.
  3. Resolving the resulting `params` map against the current template context.
  4. Invoking the declared `driver` and returning the driver's output as the placeholder value.
- Built-in functions and custom functions share the same `${name(...)}` syntax. Built-ins take precedence; a custom function with the same name as a built-in is ignored and a warning is logged.
- Custom functions run under the same capability and timeout rules as regular nodes.
- A custom function can be promoted to a built-in when it is:
  - Used in a performance-critical path.
  - Adopted widely across projects.
  - Proven safe and stable over time.

This promotion path is a core project principle: built-ins emerge from the community; they start as declarative functions and are rewritten in TypeScript when the cost of YAML interpretation matters.

## Consequences

- Contributors can add functions without a package or a merge to `handl-template-engine`.
- A healthy ecosystem of small, shareable functions can develop in any project directory and later be upstreamed.
- Custom functions are slower than built-ins because they go through the driver layer.
- The engine needs a function registry, argument binding, and timeout/capability enforcement.
- Without governance, duplicate or unsafe functions can proliferate. The promotion path and a clear deprecation policy are required.

## Alternatives considered

- **TypeScript-only functions.** Rejected. It creates too much friction for small, domain-specific helpers.
- **Dynamic JavaScript evaluation from YAML.** Rejected. It reintroduces the exact security risk the whitelisted built-in library is designed to avoid.
- **Package-based plugins for every function.** Rejected. Overhead is too high for one-off helpers.
