+++
id = "ADR-0046"
title = "ADR-0008: `handl install` and multi-registry package resolution"
status = "open"
priority = "medium"
+++

# ADR-0008: `handl install` and multi-registry package resolution

## Status

Superseded by [handl ADR-0010](../../packages/handl/project/adrs/0010-package-manager.md). The package manager and registry protocol are now a language platform concern, not Peregrin-specific.

Proposed

## Context

Peregrin workflows, patterns, drivers, and libraries need to be shared across
projects and teams. A workflow author should be able to pull in a reusable
pattern from a local directory, a GitHub repository, or any git platform. Today
the `include:` mechanism references local files, but there is no command to
discover, fetch, and install external packages.

The file router already knows how to discover workflows inside a repository. We
can reuse that idea at a higher level: a **registry** resolves a package
specifier (a string) into a local directory of Peregrin files. Once the files
are local, the file router and `include:` mechanism can take over.

We want to avoid hard-coding one source. A local file path, a GitHub `org/repo`,
a `git+https://` URL, and a GitLab `group/project` are all valid specifiers. The
CLI should resolve them through a shared abstraction, try multiple registries in
order, and install the result into the project.

## Decision

Peregrin will provide an `handl install <spec>` command and a `Registry` port. The
port resolves a specifier into a local package directory. Multiple registry
implementations can be composed into an ordered array; the first registry that
matches a specifier wins.

### Registry port (proposed)

```ts
export interface Registry {
  readonly name: string;
  readonly canResolve: (spec: string) => boolean;
  readonly resolve: (spec: string, into: string) => Effect.Effect<ResolvedPackage, RegistryError>;
}
```

`canResolve` is a cheap check based on the specifier shape. `resolve` fetches
the package and writes it into the local project at `into` (e.g. `.peregrin/` or
`patterns/`). `ResolvedPackage` carries the local path, the canonical name, and
any version or lock metadata.

### Default registry implementations (proposed)

1. **LocalFileRegistry**
   - Specifier: a relative or absolute file path such as `../patterns/deploy`
     or `/home/team/patterns`.
   - Resolution: copy or symlink the directory into the project.

2. **GitHubRegistry**
   - Specifier: `org/repo` or `github:org/repo[@ref]`.
   - Resolution: clone or sparse-checkout the repository at the given ref.
   - Default branch if no ref is provided.

3. **GitLabRegistry**
   - Specifier: `gitlab:group/project[@ref]`.
   - Resolution: same as GitHub, using GitLab clone URLs.

4. **GenericGitRegistry**
   - Specifier: `git+https://host/path[@ref]` or `git+ssh://...`.
   - Resolution: clone from any git host.

5. **URLRegistry** (future)
   - Specifier: `https://example.com/pattern.tgz`.
   - Resolution: download and extract an archive.

### Multi-registry composition

The default `InstallService` is configured with an ordered array of registries:

```ts
const defaultRegistries = [LocalFileRegistry, GitHubRegistry, GitLabRegistry, GenericGitRegistry];
```

For each `handl install` call, the service asks each registry `canResolve(spec)` in
order and uses the first that returns `true`. Users can override the array via
CLI flags or `peregrin.yaml`:

```yaml
registries:
  - github
  - gitlab
  - file
  - git
```

### Install layout

`handl install` writes packages into a project-local directory, for example
`.peregrin/packages/`. The file router and `include:` mechanism already know how
to scan directories. An installed package can be used with:

```yaml
include:
  - .peregrin/packages/9aia/patterns/deploy
```

A lockfile (`.peregrin/lock.json`) records the resolved commit, tag, or path for
each installed package so that runs are repeatable.

### Command surface (proposed)

```bash
handl install 9aia/deploy-patterns
handl install 9aia/deploy-patterns@v1.2.0
handl install filepath ../team-patterns
handl install gitlab:acme/campaigns --into patterns/shared
handl install git+https://git.example.com/corp/patterns
handl install --list
```

`--into` overrides the default install directory. `--list` shows installed
packages and their source specifiers.

## Consequences

- Reusable patterns can be shared as git repositories or local directories.
- The `include:` mechanism becomes more powerful: it can reference installed
  packages as well as local files.
- The file router gains a stable target directory for external packages.
- New packages (`handl-install`, `handl-registry-github`, `handl-registry-gitlab`,
  `handl-registry-local`) are needed to keep the CLI and individual registry
  implementations separate.
- `handl install` does not execute the workflow; it only fetches and lays out files.
- The `peregrin.yaml` project config gains a `registries` section for ordering
  and configuring registry implementations.
- The lockfile makes shared packages deterministic without vendoring the entire
  repository history.
- The install and registry system depends on ADR-0010 (Versioning) for the
  `name@version` specifier grammar and the lockfile format.

## Alternatives considered

- **Use only `include:` with full URLs.** Rejected because it forces every
  consumer to know the exact clone URL and ref. A registry abstraction hides
  that behind short specifiers.
- **Use `npm` or another package manager.** Rejected because Peregrin packages
  are file-based, not JavaScript modules, and we want the workflow author to
  think in terms of workflow files, not `package.json`.
- **A single built-in GitHub registry.** Rejected because it excludes GitLab,
  self-hosted git, and local paths. A pluggable array is more flexible.
