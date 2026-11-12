//! tamias-driver-peregrin — dual-surface driver for peregrin.
//!
//! One binary, two consumers, disjoint argv:
//!
//! **pavo view/ops driver** (P-021 "CLI-as-generated-surface" —
//! verbs take no `--dir`/`--name`):
//!
//! - `manifest` — print the [`pavo_protocol::DriverManifest`] JSON
//!   (discovery probe; never touches peregrin state).
//! - `capabilities` — merged JSON advertising BOTH surfaces: pavo's
//!   `{"capabilities": ["pavo"], "verbs": […"serve"…]}` shape plus
//!   tamias's `"types": ["peregrin-bundle","peregrin-node"]` and the
//!   provider verbs in `verbs`. pavo's discovery greps
//!   `capabilities`/`verbs` for `"pavo"`/`"serve"`; tamias's greps
//!   `types` for the content kind and warns on verbs missing from
//!   `verbs` — one payload satisfies both.
//! - `serve` — ndjson JSON-RPC on stdio (`peregrin_cli::pavo`):
//!   `{"op":"manifest"}`, `{"op":"views"}`, `{"op":"view",…}`,
//!   `{"op":"query","query":…,"params":{…}}` →
//!   `{"ok":true,"result":…}` / `{"ok":false,"error":…}`.
//!   `query:"ops"` answers the ops manifest; `query:"cli.<verb>"`
//!   runs a bounded `peregrin` CLI verb in a captured child.
//!
//! Engine surface (the `peregrin` CLI as a generated surface):
//!
//! - `ops` — the ops manifest: every bounded `peregrin` verb as a
//!   `cli.<verb>` wire query, derived from the engine's own clap
//!   tree (`peregrin_cli::cli::command`). Unbounded daemons/streams
//!   (`serve`, `campaign serve`/`run`, `sessions attach`/`stream`,
//!   `events`) are `surfaces`, never ops. Remote verbs (`sessions`,
//!   `projects`, `config`, `push`, `health`, …) are ops too — they
//!   take `--node`/`--token` params and degrade to a clean error
//!   when no node answers.
//! - `run <argv>` — engine passthrough: `tamias-driver-peregrin run
//!   version` ≡ `peregrin version`. Also the child side of `cli.*`
//!   wire queries (`engine_ops::run_verb` re-execs this same verb
//!   so engine output never touches the serve stream).
//! - `cli <peregrin argv>` — the tamias `bins[]` shim verb: `tamias
//!   bin exec` calls it with the standard `--dir`/`--name` driver
//!   flags ahead of the user's argv; they're stripped and the rest
//!   runs through `peregrin_cli::cli::run_from` in-process with
//!   inherited stdio — full engine parity (`serve` streams, remote
//!   verbs take `--node`/`--token`, `--help`/version, and every
//!   bounded verb behave exactly like the retired `peregrin`
//!   binary).
//!
//! MCP access, where needed, comes from the generic
//! `tamias-driver-mcp` surface driver (pavo) fronting this driver's
//! ops manifest — the embedded `mcp` verb was retired with
//! `peregrin-mcp` (P-028/T-021); `cli` needs no feature: it's the
//! bins-shim passthrough above, not a generated surface — the
//! engine's own argv grammar is the contract.
//!
//! **tamias content-type driver** (`type: peregrin-bundle` /
//! `peregrin-node` packages — tamias always passes `--dir <payload>
//! --name <pkg> [--config <f>]`, so these verbs are unreachable
//! from pavo's bare probes):
//!
//! - `install` — no-op — node config materialization is `deploy`'s
//!   job (it sees the stable store dir; install only sees the
//!   evictable fetch-cache payload).
//! - `deploy` — regenerate the operator node config from the
//!   manifest's `provider:` block (provider.services units are
//!   rendered by `tamias deploy`, not this verb).
//! - `uninstall` — remove the node config + host config + any
//!   legacy unit. `--dir` accepted for dispatch parity, ignored.
//! - `hooks register|unregister [--dry-run] [--api-base <url>]
//!   [--token-env <NAME>]` — upsert/remove the forge webhooks the
//!   manifest's `provider.ingress[].hooks[]` declare.
//! - `health` — JSON probe: github-webhook `ping` per ingress
//!   `listen`+`path`, else `systemctl --user is-active` per unit.
//! - `status` — JSON per-unit `is-active`/`is-enabled`.
//! - `logs [--lines N]` — `journalctl --user` over the provider units.
//! - `config-schema` — reserved verb: the JSON Schema (draft
//!   2020-12) of the node config `deploy` writes.
//!
//! Thin wrapper over `peregrin_cli::{provision, ops, engine_ops,
//! pavo}`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use peregrin_cli::{engine_ops, ops, pavo, provision};

#[derive(Parser)]
#[command(name = "tamias-driver-peregrin", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// No-op — node config materialization is `deploy`'s job (it sees
    /// the stable store dir; install only sees the fetch cache).
    Install {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Regenerate the operator node config (provider.services units
    /// are rendered by `tamias deploy`, not this verb).
    Deploy {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Remove the node config + host config + any legacy unit. `--dir`
    /// is accepted for dispatch parity (tamias always passes it) and
    /// ignored — uninstall resolves paths from `--name`.
    Uninstall {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Manage the forge webhooks `provider.ingress[].hooks[]` declare —
    /// `tamias driver exec <pkg> hooks register|unregister`.
    Hooks {
        /// `register` upserts; `unregister` removes.
        action: HookAction,
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
        /// Print the plan — no forge API calls, no secret resolution.
        #[arg(long)]
        dry_run: bool,
        /// Forge REST base (default `https://api.github.com`).
        #[arg(long)]
        api_base: Option<String>,
        /// Forge-token secret NAME (default `GITHUB_TOKEN`).
        #[arg(long)]
        token_env: Option<String>,
    },
    /// Probe the provider — JSON `{ok, probes[]}`: a github-webhook
    /// `ping` per ingress `listen`+`path`, else unit `is-active`.
    Health {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Per-unit `systemctl --user is-active`/`is-enabled` — JSON.
    Status {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// `journalctl --user` over the provider's units.
    Logs {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
        /// Journal lines per unit.
        #[arg(long, default_value_t = 200)]
        lines: u32,
    },
    /// Print the JSON Schema (draft 2020-12) of the `--config` node
    /// config this driver generates — tamias's reserved `config-schema`
    /// verb. Instance-agnostic: the flags are accepted for dispatch
    /// parity and ignored.
    ConfigSchema {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum HookAction {
    /// Upsert the manifest's forge hooks (idempotent — matches on the
    /// exact hook URL).
    Register,
    /// Remove the hooks the manifest declares.
    Unregister,
}

/// `cli` verb argv (after the `cli` token) → engine argv (argv[0]
/// included). `tamias bin exec`/`driver exec` prepend the standard
/// driver flags — `--dir <entry>` `--name <pkg>` — which the engine
/// doesn't know; consume the leading run of those two (`--flag v`
/// and `--flag=v` forms) and pass everything else through untouched.
/// `--config` is deliberately NOT stripped: it's not a peregrin
/// global today, but a user-passed `--config` must reach whatever
/// verb owns it.
fn engine_argv(args: &[String]) -> Vec<String> {
    let mut out = vec!["peregrin".to_string()];
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--dir" || a == "--name" {
            // flag + separate value — skip both (a trailing flag
            // with no value just ends the scan).
            i = (i + 2).min(args.len());
        } else if a.starts_with("--dir=") || a.starts_with("--name=") {
            i += 1;
        } else {
            break;
        }
    }
    out.extend(args[i..].iter().cloned());
    out
}

fn main() -> ExitCode {
    // pavo/tamias-shared verbs dispatch on argv[1] before clap —
    // pavo's probes are bare (`manifest`, `capabilities`, `serve`)
    // while tamias always passes `--dir`/`--name`, so the surfaces
    // stay disjoint.
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match argv.first().map(String::as_str) {
        Some("manifest") => {
            println!("{}", pavo::manifest_json());
            return ExitCode::SUCCESS;
        }
        Some("capabilities") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&pavo::capabilities_value()).unwrap_or_default()
            );
            return ExitCode::SUCCESS;
        }
        Some("ops") => {
            println!("{}", engine_ops::manifest());
            return ExitCode::SUCCESS;
        }
        // Engine passthrough — `tamias-driver-peregrin run version` is
        // `peregrin version`. Also the child side of `cli.*` wire
        // queries (engine_ops::run_verb re-execs this same verb) and
        // the streaming surface for `surfaces` verbs (`run serve`,
        // `run campaign serve`, …).
        Some("run") => {
            let argv = std::iter::once("peregrin".to_string()).chain(argv.iter().skip(1).cloned());
            std::process::exit(peregrin_cli::cli::run_from(argv));
        }
        // The pavo wire loop — `{"op":"query","query":"cli.<verb>"}`
        // etc. over ndjson JSON-RPC on stdio.
        Some("serve") => std::process::exit(pavo::serve()),
        // The tamias `bins[]` shim verb (T-012): `tamias bin exec`
        // runs `<driver> cli --dir <entry> --name <pkg> <peregrin
        // argv>` for a declared `shim: cli` bin. peregrin's engine IS
        // the CLI, so this is a passthrough — strip the two standard
        // driver flags and hand the rest to `cli::run_from` with
        // inherited stdio. That keeps full engine parity a generated
        // ops→CLI surface can't express: every verb, `--help`/
        // `--version`, the unbounded `serve`/`campaign serve`/
        // `sessions attach` surfaces, and remote verbs' `--node`/
        // `--token` flags all behave exactly like the retired
        // `peregrin` binary.
        Some("cli") => {
            let argv = engine_argv(&argv[1..]);
            std::process::exit(peregrin_cli::cli::run_from(argv));
        }
        _ => {}
    }
    // Tamias verbs — clap owns parse errors/`--help` exit codes.
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Install { .. } => {
            println!("install: node config is generated by `tamias deploy` — nothing to do");
            return ExitCode::SUCCESS;
        }
        Cmd::Deploy { dir, name, config } => provision::deploy(&dir, &name, config.as_deref()),
        Cmd::Uninstall { name, config, .. } => provision::uninstall(&name, config.as_deref()),
        Cmd::Hooks {
            action,
            dir,
            name: _,
            config: _,
            dry_run,
            api_base,
            token_env,
        } => provision::hooks(
            &dir,
            &ops::OpsEnv::default(),
            matches!(action, HookAction::Register),
            dry_run,
            api_base.as_deref(),
            token_env.as_deref(),
        ),
        Cmd::Health { dir, name, .. } => provision::health(&dir, &name),
        Cmd::Status { dir, name, .. } => provision::status(&dir, &name),
        Cmd::Logs {
            dir, name, lines, ..
        } => provision::logs(&dir, &name, lines),
        Cmd::ConfigSchema { .. } => provision::config_schema(),
    };
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("tamias-driver-peregrin: {e}");
            ExitCode::FAILURE
        }
    }
}
