# ADR-0003: Secrets provider abstraction

## Status

Proposed

## Context

Workflows need secrets for API keys, tokens, agent credentials, and environment-specific values. Today, `ac-file-router` discovers `.secret.yml` and `.secret.yaml` files and passes them as a `Record<string, string>` into the `Workflow`. The `Runner` receives `secrets` directly, and the template engine exposes them as `secret.*` placeholders. The `ExecutionContext.getSecret` method reads from this in-memory record.

This works for local development and small repositories, but it does not scale to production:

- `.secret.yml` files end up in version control or require `.gitignore` gymnastics.
- Cloud-native teams want to read from AWS Secrets Manager, GCP Secret Manager, Azure Key Vault, Doppler, 1Password, etc.
- Local development often uses `.env` files or shell environment variables.
- CI systems may inject secrets as environment variables or mounted JSON files.

The current design hard-codes `.secret.yml` as the loader. There is no interface to plug in a different secret source, and the file router is doing two jobs: file discovery and secret extraction.

## Decision

Introduce a `SecretStore` port in `ac-core` following the same pattern as `EventStore` and `Observer`:

```ts
export interface SecretStore {
  readonly get: (key: string) => Effect.Effect<string | undefined, SecretError>;
  readonly has: (key: string) => Effect.Effect<boolean, never>;
  readonly all: () => Effect.Effect<Readonly<Record<string, string>>, SecretError>;
  readonly close: () => Effect.Effect<void, never>;
}

export const SecretStore = Context.GenericTag<SecretStore>("ac/SecretStore");
```

The `Workflow` and `Runner` no longer receive a raw `secrets` record. Instead, the runner requires a `SecretStore` layer. The `ExecutionContext.getSecret` and `ctx.secrets` now read from this store.

### Default implementations

- `ac-secrets-yaml` — the current behaviour, loading `.secret.yml` / `.secret.yaml` discovered by the file router. This remains the default for local/CLI runs.
- `ac-secrets-dotenv` — loads secrets from `.env` and process environment variables.
- `ac-secrets-json` — loads secrets from a JSON file passed via `--secrets-file`.

### Future implementations

- `ac-secrets-gcp` — Google Cloud Secret Manager.
- `ac-secrets-aws` — AWS Secrets Manager.
- `ac-secrets-azure` — Azure Key Vault.
- `ac-secrets-doppler` — Doppler.

### Loading rules

- The CLI and programmatic API accept `--secrets-provider` or `secretsProvider` configuration.
- The default provider is `yaml` for backwards compatibility, specifically `.secret.yml` / `.secret.yaml` in the workflow directory.
- Multiple providers can be composed: `yaml` first, then `dotenv`, then `json`. The first provider to return a value wins. Composition is handled by a `CompositeSecretStore` in `ac-core`.
- If `strict: true` (see ADR-0001), a missing secret is a runtime error. If `strict: false`, `getSecret` returns the default value or an empty string.

The `ac-file-router` will continue to discover `.secret.yml` files, but it will only return the file paths and any inline `secrets` map from `config`. The actual resolution will be done by the selected `SecretStore` implementation at runtime.

## Consequences

- Secrets are no longer collected into a single in-memory record during file discovery. They are resolved lazily and can be backed by any provider.
- The `Runner` and `ExecutionContext` need to receive a `SecretStore` layer rather than a `secrets` record.
- The `ac-cli-defaults` default stack needs to include `SecretStoreLive` using the `yaml` provider by default.
- Existing `.secret.yml` workflows keep working without changes.
- New CLI flags (`--secrets-provider`, `--secrets-file`, `--env-file`) are needed.
- Security improves: cloud secret managers can be used without storing plain values on disk.
- Tests can use a `MemorySecretStore` from `ac-testing-library` instead of writing `.secret.yml` files.

## Alternatives considered

- **Keep `.secret.yml` as the only source and add a pre-processing step that fetches external secrets into a local file.** This still leaves the project with secrets on disk and does not solve the version-control or rotation problems.
- **Make secrets part of the `EventStore` or `Observer` ports.** Secrets are a different concern with different access patterns (read-only, sensitive, possibly remote). A dedicated port is cleaner.
- **Pass a raw `Record<string, string>` but allow the CLI to populate it from any source.** This keeps the engine simple but pushes the abstraction into the CLI, making it impossible for drivers and tests to use the same provider code.
