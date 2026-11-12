//! Engine ops — `cli.<verb>` wire queries dispatch into the
//! `peregrin` engine lib through this binary's own `run` verb:
//! each call re-execs `tamias-driver-peregrin run <argv>` in a
//! captured child, so `peregrin_cli::cli::run_from` writes to the
//! child's stdout and never touches the serve loop's ndjson
//! stream.
//!
//! `ops` answers the ops manifest as JSON — *derived from the
//! engine's own clap tree* ([`crate::cli::command`]), so a verb
//! can't exist on the CLI and be missing here: declared args
//! become typed params (`arg`/`opt`/`flag`), doc strings ride
//! along unchanged. The CLI has no globals — the node target
//! (`--node`/`--token`, `PEREGRIN_NODE_URL`/`PEREGRIN_TOKEN`) is a
//! per-verb param block, so remote ops carry it explicitly.
//!
//! Unbounded/host-interactive verbs can never be request/response
//! ops — they never return or need inherited stdio — so they ride
//! `surfaces` instead: surface drivers (tamias-driver-cli,
//! tamias-driver-mcp, the pavo-driver-kit `surface` layer)
//! expose them as streaming subcommands — `tamias-driver-peregrin
//! cli serve` ≡ `peregrin serve`.
//!
//! `mutates` is honest: remote verbs write node state (sessions,
//! projects, config, push subs, pairing tokens), `pair`/`import`
//! write local files, `campaign run` executes workflows with
//! driver side effects. Read-only ops are the probes and history
//! readers below. Remote ops degrade gracefully — with no node at
//! `--node` the child exits 1 with the transport error in
//! `stderr`, honest data like any other nonzero `code`.

use clap::{Arg, ArgAction, Command};
use pavo_protocol::wire::{QueryResult, Response};
use serde_json::{Map, Value, json};

/// Unbounded/host-interactive surfaces — verbs that never return
/// or stream/attach indefinitely. They can never be
/// request/response ops; they need inherited stdio.
///
/// - `serve` — the peregrin node daemon itself.
/// - `campaign.serve` — the multi-campaign webhook listener.
/// - `campaign.run` — bounded for a plain run, but `--queue-store`
///   supervisor mode never returns and campaigns can run
///   arbitrarily long; the honest home is the streaming surface.
/// - `sessions.attach` — interactive live control of a session.
/// - `sessions.stream` / `events` — live event streams.
const SURFACES: &[&str] = &[
    "serve",
    "campaign.run",
    "campaign.serve",
    "sessions.attach",
    "sessions.stream",
    "events",
];

/// Params excluded from ops — flags that turn a bounded verb into
/// a stream. None today (the streaming verbs are surfaces
/// wholesale).
const SKIP_PARAMS: &[&str] = &[];

/// Read-only verbs — everything else writes node state, local
/// files, or executes workflows. Mutators belong behind a
/// deliberate trigger, not ambient view polling.
const READ_ONLY: &[&str] = &[
    "version",
    "runs",
    "log",
    "explain",
    "health",
    "node",
    "agents",
    "user",
    "fs",
    "projects.list",
    "config.get",
    "push.vapid",
    "campaign.validate",
    "campaign.status",
    "campaign.events",
    "bundle.list",
    "bundle.info",
    "logs.runs",
    "logs.events",
    "logs.explain",
    "sessions.list",
    "sessions.history",
    "sessions.checkpoints",
    "sessions.export",
];

/// `(name, clap spec)` for every leaf verb. Hidden plumbing
/// (`provision`) and clap's `help` are skipped — they stay
/// reachable through `run` passthrough.
fn verbs() -> Vec<(String, Command)> {
    fn walk(prefix: &str, cmd: &Command, out: &mut Vec<(String, Command)>) {
        for sub in cmd.get_subcommands() {
            let name = sub.get_name();
            if name == "help" || sub.is_hide_set() {
                continue;
            }
            let dotted = match prefix {
                "" => name.to_string(),
                _ => format!("{prefix}.{name}"),
            };
            if sub.has_subcommands() {
                walk(&dotted, sub, out);
            } else {
                out.push((dotted, sub.clone()));
            }
        }
    }
    let mut out = Vec::new();
    walk("", &crate::cli::command(), &mut out);
    out
}

fn op_cmd(name: &str) -> Option<Command> {
    verbs()
        .into_iter()
        .find(|(n, _)| n == name && !SURFACES.contains(&n.as_str()))
        .map(|(_, c)| c)
}

/// Args that can't ride the wire: streaming flags (above) and
/// clap's own help/version actions. peregrin has no globals —
/// `--node`/`--token` are per-verb params.
fn skipped(a: &Arg) -> bool {
    SKIP_PARAMS.contains(&a.get_id().as_str())
        || matches!(
            a.get_action(),
            ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
        )
}

/// A clap arg → the manifest's typed param kind — `arg` = required
/// positional, `opt` = `--name <v>` *or* an optional positional
/// (the generated surfaces spell it `--name`; `argv` still lands
/// it positionally), `flag` = `--name`.
fn kind(a: &Arg) -> &'static str {
    if a.is_positional() {
        if a.is_required_set() { "arg" } else { "opt" }
    } else if matches!(
        a.get_action(),
        ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count
    ) {
        "flag"
    } else {
        "opt"
    }
}

fn param_json(a: &Arg) -> Value {
    // Wire name = the flag name (`--dry-run` → `dry-run`), not the
    // field id (`dry_run`) — generated surfaces spell params
    // `--<name>`.
    let id = a.get_id().as_str();
    let name = if a.is_positional() {
        id
    } else {
        a.get_long().unwrap_or(id)
    };
    // Bearer tokens are secret-bearing params — surfaces steer
    // callers to `--params-stdin` instead of argv (pavo surface
    // contract; older pavo revs ignore the marker).
    let secret = matches!(name, "token" | "source-token" | "secrets");
    let mut p = json!({
        "name": name,
        "kind": kind(a),
        "doc": a.get_help().map(ToString::to_string).unwrap_or_default(),
    });
    if secret {
        p["secret"] = json!(true);
    }
    p
}

/// The `ops` query/`ops` argv verb payload — self-describing so
/// hosts and agents discover the engine surface without argv help.
/// Remote ops take the node target as ordinary params (`--node`,
/// `--token`, `--agent`, `--json` on the `NodeArgs`/`TargetArgs`
/// blocks); env defaults (`PEREGRIN_NODE_URL`/`PEREGRIN_TOKEN`)
/// apply inside the child when they're absent.
pub fn manifest() -> Value {
    json!({
        "engine": "peregrin",
        "invoke": "query `cli.<verb>` with the listed params, or argv `tamias-driver-peregrin run <peregrin argv>`",
        "ops": verbs()
            .iter()
            .filter(|(n, _)| !SURFACES.contains(&n.as_str()))
            .map(|(n, c)| json!({
                "query": format!("cli.{n}"),
                "doc": c.get_about().map(ToString::to_string).unwrap_or_default(),
                "params": c.get_arguments()
                    .filter(|a| !skipped(a) && !a.is_global_set())
                    .map(param_json)
                    .collect::<Vec<_>>(),
                "mutates": !READ_ONLY.contains(&n.as_str()),
            }))
            .collect::<Vec<_>>(),
        "surfaces": SURFACES,
    })
}

/// Is `query` an engine op (`cli.<verb>`)? Returns the verb name.
pub fn op_of(query: &str) -> Option<String> {
    query
        .strip_prefix("cli.")
        .filter(|n| op_cmd(n).is_some())
        .map(str::to_string)
}

/// A wire scalar → its argv token (numbers/bools stringify).
fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// All values of a param — arrays flatten, scalars wrap.
fn values(v: &Value) -> Vec<String> {
    match v {
        Value::Array(xs) => xs.iter().filter_map(scalar).collect(),
        other => scalar(other).into_iter().collect(),
    }
}

/// Emit `--<long> <v>` / `--<long>` for one arg when `params`
/// carries it — looked up by wire name (long flag), field id, and
/// kebab-case id, so callers may spell params either way. Flags
/// read truthiness; opts/args take every value.
fn push_param(out: &mut Vec<String>, a: &Arg, params: &Map<String, Value>) {
    let id = a.get_id().as_str();
    let long = a.get_long().unwrap_or(id);
    let kebab = id.replace('_', "-");
    let raw = params
        .get(long)
        .or_else(|| params.get(id))
        .or_else(|| params.get(kebab.as_str()));
    let Some(raw) = raw else { return };
    match kind(a) {
        "flag" => {
            if raw.as_bool().unwrap_or(false) {
                out.push(format!("--{long}"));
            }
        }
        _ => {
            for s in values(raw) {
                out.push(format!("--{long}"));
                out.push(s);
            }
        }
    }
}

/// `cli.<verb>` params → peregrin argv (no argv[0] — `run`
/// prepends it). Named flags first, then positionals in schema
/// order. peregrin has no globals.
fn argv(name: &str, cmd: &Command, params: &Map<String, Value>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    out.extend(name.split('.').map(String::from));
    for a in cmd.get_arguments() {
        if skipped(a) || a.is_positional() || a.is_global_set() {
            continue;
        }
        push_param(&mut out, a, params);
    }
    // `trailing_var_arg`/`last` positionals only parse after a
    // `--` separator — none today, kept generic.
    if cmd.get_positionals().any(Arg::is_last_set) {
        out.push("--".into());
    }
    let mut positionals: Vec<String> = Vec::new();
    for a in cmd.get_positionals() {
        if skipped(a) {
            continue;
        }
        let id = a.get_id().as_str();
        let raw = params
            .get(id)
            .or_else(|| params.get(id.replace('_', "-").as_str()));
        if let Some(raw) = raw {
            positionals.extend(values(raw));
        }
    }
    out.extend(positionals);
    out
}

/// Run one engine op by name: re-exec `run` in a child with
/// captured output, and hand back `{op, argv, code, stdout,
/// stderr}` as the query payload. `ok:false` is reserved for
/// unknown ops and spawn failures — a nonzero `code` is the
/// engine's own exit status, honest data (a dead node surfaces as
/// `code: 1` + the transport error on `stderr`). The child's
/// stdin is null and it inherits this process's cwd/env —
/// unbounded/host-interactive verbs (`serve`, `campaign serve`/
/// `run`, `sessions attach`/`stream`, `events`) are surfaces,
/// never ops, precisely because they need inherited stdio.
pub fn run_verb(name: &str, params: &Map<String, Value>) -> Response {
    let name = name.strip_prefix("cli.").unwrap_or(name);
    let Some(cmd) = op_cmd(name) else {
        return Response::err(format!("unknown op: {name}"));
    };
    let args = argv(name, &cmd, params);
    let out = std::env::current_exe().and_then(|exe| {
        std::process::Command::new(exe)
            .arg("run")
            .args(&args)
            .output()
    });
    let data = match out {
        Ok(o) => json!({
            "op": name,
            "argv": args,
            "code": o.status.code().unwrap_or(-1),
            "stdout": String::from_utf8_lossy(&o.stdout),
            "stderr": String::from_utf8_lossy(&o.stderr),
        }),
        Err(e) => return Response::err(format!("run {name}: {e}")),
    };
    Response::ok_typed(&QueryResult {
        query: format!("cli.{name}"),
        data,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    #[test]
    fn op_of_accepts_cli_prefix_only() {
        assert_eq!(op_of("cli.version").unwrap(), "version");
        assert_eq!(op_of("cli.sessions.list").unwrap(), "sessions.list");
        assert!(op_of("cli.serve").is_none()); // unbounded surface
        assert!(op_of("cli.campaign.serve").is_none()); // listener surface
        assert!(op_of("cli.campaign.run").is_none()); // supervisor-capable surface
        assert!(op_of("cli.sessions.attach").is_none()); // interactive surface
        assert!(op_of("cli.sessions.stream").is_none()); // stream surface
        assert!(op_of("cli.events").is_none()); // stream surface
        assert!(op_of("cli.provision").is_none()); // hidden plumbing
        assert!(op_of("cli.sessions").is_none()); // group, not a leaf
        assert!(op_of("version").is_none()); // view query, not an op
        assert!(op_of("cli.bogus").is_none());
    }

    #[test]
    fn manifest_lists_every_non_surface_verb_as_cli_query() {
        let m = manifest();
        let ops = m["ops"].as_array().unwrap();
        for (n, _) in verbs()
            .iter()
            .filter(|(n, _)| !SURFACES.contains(&n.as_str()))
        {
            assert!(
                ops.iter().any(|o| o["query"] == json!(format!("cli.{n}"))),
                "{n} missing from ops manifest"
            );
        }
        assert_eq!(m["surfaces"], json!(SURFACES));
        for o in ops {
            for p in o["params"].as_array().unwrap() {
                assert!(["arg", "opt", "flag"].contains(&p["kind"].as_str().unwrap()));
                // streaming params never ride the wire
                assert!(!SKIP_PARAMS.contains(&p["name"].as_str().unwrap()));
            }
        }
        // mutates honesty — the read-only set is exactly these
        let mut readers: Vec<&str> = ops
            .iter()
            .filter(|o| o["mutates"] == json!(false))
            .filter_map(|o| o["query"].as_str().map(|q| q.strip_prefix("cli.").unwrap()))
            .collect();
        readers.sort_unstable();
        let mut expected = READ_ONLY.to_vec();
        expected.sort_unstable();
        assert_eq!(readers, expected);
    }

    #[test]
    fn argv_maps_params_to_peregrin_flags() {
        let cmd = op_cmd("sessions.meta").unwrap();
        let mut p = Map::new();
        p.insert("session_id".into(), json!("s1"));
        p.insert("title".into(), json!("new title"));
        p.insert("pin".into(), json!(true));
        p.insert("node".into(), json!("http://127.0.0.1:9"));
        assert_eq!(
            argv("sessions.meta", &cmd, &p),
            vec![
                "sessions",
                "meta",
                "--title",
                "new title",
                "--pin",
                "--node",
                "http://127.0.0.1:9",
                "s1"
            ]
        );
    }

    #[test]
    fn argv_omits_absent_params() {
        let cmd = op_cmd("version").unwrap();
        let p = Map::new();
        assert_eq!(argv("version", &cmd, &p), vec!["version"]);
    }
}
