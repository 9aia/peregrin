# `handl-cli`

Command-line interface for the handl language. The `handl` binary evaluates, compiles, and type-checks handl programs.

## Build

From the workspace root:

```sh
cargo build -p handl-cli
```

The binary is available as `target/debug/handl` (or `target/release/handl` with `--release`).

## Usage

```sh
handl <COMMAND> [OPTIONS] [INPUT]...
```

Input files are passed as positional arguments. If none are given, `stdin` is read once.

## Commands

| Command     | Purpose                                                 |
| ----------- | ------------------------------------------------------- |
| `eval`      | Evaluate one or more expressions and print the results. |
| `compile`   | Compile a single handl expression to a target language. |
| `typecheck` | Static type-check one or more `$:` declarations.        |

Run `handl --help` or `handl <COMMAND> --help` for full option details.

## Options

### `eval`

- `INPUT...` — Input files. Defaults to `stdin` if omitted.
- `-f, --format <handl|toml>` — Input format. Defaults to `handl`.
- `-o, --output <toml>` — Output format. Defaults to `toml`.
- `-v, --vm <sandbox|server>` — VM host. Defaults to `sandbox`.

### `compile`

- `INPUT` — Input file. Defaults to `stdin` if omitted.
- `-f, --format <handl>` — Input format. Defaults to `handl`.
- `-o, --output <PATH>` — Output file. Writes to `stdout` if omitted.
- `-t, --target <js>` — Target language. Currently only `js` is supported.

### `typecheck`

- `INPUT...` — Input files. Defaults to `stdin` if omitted.
- `-f, --format <handl|toml>` — Input format. Defaults to `handl`.

## Examples

```sh
# Evaluate a single handl file
handl eval program.handl

# Evaluate multiple handl files
handl eval file1.handl file2.handl

# Evaluate a TOML file
handl eval config.toml -f toml

# Compile an handl expression to JavaScript
handl compile program.handl -t js -o out.js

# Static type-check an handl file
handl typecheck program.handl

# Static type-check multiple files
handl typecheck file1.handl file2.handl
```

## Tests

```sh
cargo test -p handl-cli
```
