#![allow(unknown_lints, clippy::assert_is_empty)]

//! peregrin — the CLI. Local store verbs over the
//! Devin/Cline/Claude/Cursor stores, node ops against a running node's
//! `/api/*` surface, agent config IR verbs, pairing, `serve` (the node
//! itself, shared with `peregrin-node`), and `service` management
//! (systemd/launchd).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};

use peregrin_cli::node_ops::{NodeArgs, TargetArgs};
use peregrin_cli::{
    CliError, PEREGRIN_VERSION, api, campaign, distribution, import, node_ops, ops, pair, service,
};

fn default_peregrin_home() -> PathBuf {
    peregrin_node::peregrin_home()
}

fn default_pair_url() -> String {
    let port = std::env::var("PORT").unwrap_or_else(|_| "18787".into());
    format!("http://localhost:{port}")
}

/// `--from`/`--to` choices for `sessions convert|import|resume` — only
/// cline/devin are writable targets on the node.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum ImportTo {
    Cline,
    Devin,
}

impl ImportTo {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cline => "cline",
            Self::Devin => "devin",
        }
    }
}

// --- sessions --------------------------------------------------------------

/// `--agent` — scope the session id lookup to this agent's store (ids
/// collide across agents).
#[derive(Clone, Args)]
struct AgentQuery {
    /// Scope the session id lookup to this agent's store — ids collide across agents
    #[arg(long)]
    agent: Option<String>,
}

/// `session-id` positional's help text.
const SESSION_ID_HELP: &str = "Session id on the target node";

#[derive(Subcommand)]
enum SessionsCommands {
    /// GET /api/sessions — list the node's sessions
    List {
        /// Also probe each agent for live lock state (?withLocks=1)
        #[arg(long)]
        locks: bool,
        #[command(flatten)]
        args: NodeArgs,
    },
    /// POST /api/sessions — create a session
    Create {
        /// Working directory the agent session runs in
        #[arg(long, default_value_os_t = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))]
        cwd: PathBuf,
        /// Agent runtime to spawn (default: the node's default agent)
        #[arg(long)]
        agent: Option<String>,
        /// Session title
        #[arg(long)]
        title: Option<String>,
        /// Preferred model
        #[arg(long)]
        model: Option<String>,
        /// Fallback model — repeatable, applied in order
        #[arg(long)]
        fallback: Vec<String>,
        #[command(flatten)]
        args: NodeArgs,
    },
    /// POST /api/sessions/:id/attach — attach live control (read-only
    /// while another process holds the lock)
    Attach {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// SIGTERM the lock holder and attach writable
        #[arg(long)]
        takeover: bool,
        /// Preferred model for this attach
        #[arg(long)]
        model: Option<String>,
        /// Fallback model — repeatable
        #[arg(long)]
        fallback: Vec<String>,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: NodeArgs,
    },
    /// POST /api/sessions/:id/prompt — send one turn to an attached session
    Prompt {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// The prompt text to send
        text: String,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// POST /api/sessions/:id/cancel — stop the current run
    Cancel {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// POST /api/sessions/:id/permission — answer a pending permission request
    Permission {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// The pending permission request id (requestId)
        #[arg(long)]
        request: String,
        /// The option id to accept; omit to decline the request
        #[arg(long)]
        option: Option<String>,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// GET /api/sessions/:id/history — the paginated backlog
    History {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// Number of trailing messages to return
        #[arg(long)]
        limit: Option<i64>,
        /// Exclusive end index — page backwards through the backlog
        #[arg(long)]
        before: Option<i64>,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: NodeArgs,
    },
    /// GET /api/sessions/:id/checkpoints — workspace snapshot refs the
    /// store recorded
    Checkpoints {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: NodeArgs,
    },
    /// GET /api/sessions/:id/export — the complete session IR (feeds
    /// `sessions import`)
    Export {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// Write the session IR JSON to this file (default: stdout)
        #[arg(long)]
        out: Option<String>,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// GET /api/sessions/:id/stream — stream the live run's session
    /// events (sepia-proto SessionEvent JSON) to stdout
    Stream {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// PATCH /api/sessions/:id — the meta overlay (title, pinned,
    /// archived, projects, model)
    Meta {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// Rename the session
        #[arg(long)]
        title: Option<String>,
        /// Pin the session
        #[arg(long)]
        pin: bool,
        /// Unpin the session
        #[arg(long)]
        unpin: bool,
        /// Archive the session
        #[arg(long)]
        archive: bool,
        /// Unarchive the session
        #[arg(long)]
        unarchive: bool,
        /// Project ids to assign — repeatable; replaces the whole list
        #[arg(long)]
        project: Vec<String>,
        /// Recorded model override
        #[arg(long)]
        model: Option<String>,
        /// Clear the recorded model override
        #[arg(long)]
        clear_model: bool,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// PATCH /api/sessions/:id — rename a session
    Rename {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// The new title
        title: String,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// DELETE /api/sessions/:id
    Delete {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// POST /api/sessions/:id/convert — convert a session into another
    /// agent's store on the node
    Convert {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// Agent store to convert into
        #[arg(long)]
        to: ImportTo,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// POST /api/sessions/import — write session IR (or a history array)
    /// into an agent store
    Import {
        /// A session JSON export (`sessions export` / `peregrin export`
        /// output) or a history array
        path: PathBuf,
        /// Agent store to write into
        #[arg(long)]
        to: ImportTo,
        /// Override the imported working directory
        #[arg(long)]
        cwd: Option<String>,
        /// Override the imported title
        #[arg(long)]
        title: Option<String>,
        /// Override the imported model
        #[arg(long)]
        model: Option<String>,
        #[command(flatten)]
        args: NodeArgs,
    },
    /// "Resume on…" — pull a session from the source node and write it
    /// into an agent store here
    Resume {
        /// Session id on the source node
        session_id: String,
        /// Agent store on the target node to resume into
        #[arg(long)]
        to: ImportTo,
        /// Source node URL (default: same as --node)
        #[arg(long, default_value_t = api::default_node_url())]
        source: String,
        /// Bearer token for the source node (default: --token)
        #[arg(long)]
        source_token: Option<String>,
        /// Scope the source lookup to this agent's store
        #[arg(long)]
        from_agent: Option<String>,
        /// Override the working directory on the target
        #[arg(long)]
        cwd: Option<String>,
        /// Override the title on the target
        #[arg(long)]
        title: Option<String>,
        /// Override the model on the target
        #[arg(long)]
        model: Option<String>,
        #[command(flatten)]
        args: NodeArgs,
    },
    /// POST /api/sessions/:id/restore — revert workspace files via
    /// recorded diffs or a checkpoint ref
    Restore {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// Required — restore writes real files under the session's cwd
        #[arg(long)]
        confirm: bool,
        /// File to revert through the session's recorded diffs
        #[arg(long)]
        path: Option<String>,
        /// Revert only this tool call's change to --path
        #[arg(long)]
        tool_call_id: Option<String>,
        /// Materialize a recorded snapshot ref (see `sessions checkpoints`)
        #[arg(long)]
        checkpoint: Option<String>,
        /// Narrow a --checkpoint restore to these files — repeatable
        #[arg(long)]
        paths: Vec<String>,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: NodeArgs,
    },
    /// POST /api/sessions/:id/rewind — truncate the transcript at a node,
    /// turn count or checkpoint
    Rewind {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// Required — rewind deletes stored history
        #[arg(long)]
        confirm: bool,
        /// Keep this history node and everything before it
        #[arg(long)]
        node_id: Option<i64>,
        /// Drop the last N user turns
        #[arg(long)]
        turns: Option<i64>,
        /// Rewind to a recorded snapshot ref
        #[arg(long)]
        checkpoint: Option<String>,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: NodeArgs,
    },
}

// --- projects ----------------------------------------------------------------

#[derive(Subcommand)]
enum ProjectsCommands {
    /// GET /api/projects
    List {
        #[command(flatten)]
        args: NodeArgs,
    },
    /// POST /api/projects — create a project
    Create {
        name: String,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// Alias of `projects create` — init a project (repo) on the node
    Init {
        name: String,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// PATCH /api/projects/:id
    Rename {
        project_id: String,
        name: String,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// DELETE /api/projects/:id
    Delete {
        project_id: String,
        #[command(flatten)]
        args: TargetArgs,
    },
}

// --- config --------------------------------------------------------------------

#[derive(Subcommand)]
enum ConfigCommands {
    /// GET /api/config — the node's server-side UI state
    Get {
        #[command(flatten)]
        args: NodeArgs,
    },
    /// PATCH /api/config/:key
    Set {
        key: String,
        /// JSON when parseable, else a plain string
        value: String,
        #[command(flatten)]
        args: TargetArgs,
    },
}

// --- campaigns -----------------------------------------------------------------

/// Campaign verbs — the `handl` CLI surface, on this machine's
/// campaign plane (`peregrin_node::campaign`, in-process).
#[derive(Subcommand)]
enum CampaignCommands {
    /// Load a campaign yaml and execute it locally — prints the run's
    /// event log, one `ts source type` line per event
    Run {
        /// Path to the campaign YAML
        workflow: PathBuf,
        /// Simulate the run without driver side effects
        #[arg(long)]
        dry_run: bool,
        /// A JSON/JSONL event log to seed (replay) the run with
        #[arg(long)]
        replay: Option<PathBuf>,
        /// Persist the run's event log to this JSONL file (ADR-0040's
        /// --history; `peregrin runs`/`explain`/`log` read it back)
        #[arg(long)]
        history: Option<PathBuf>,
    },
    /// Expand a campaign yaml and summarize its wiring (includes,
    /// nodes, unemitting-subscription warnings)
    Validate {
        /// Path to the campaign YAML
        workflow: PathBuf,
    },
}

// --- push --------------------------------------------------------------------

#[derive(Subcommand)]
enum PushCommands {
    /// GET /api/push/vapid — the node's web-push public key
    Vapid {
        #[command(flatten)]
        args: TargetArgs,
    },
    /// POST /api/push/subscribe
    Subscribe {
        /// A push subscription JSON ({endpoint, keys:{auth,p256dh}, prefs?})
        subscription: String,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// DELETE /api/push/subscribe
    Unsubscribe {
        endpoint: String,
        #[command(flatten)]
        args: TargetArgs,
    },
}

// --- store / service ----------------------------------------------------------

#[derive(Subcommand)]
enum ServiceCommands {
    /// Install the peregrin node (or hub, --hub) as an OS service
    /// (systemd/launchd)
    Install {
        /// Manage the hub unit (peregrin-hub) instead of the node unit
        #[arg(long)]
        hub: bool,
        /// Install/manage the system-level unit instead of the per-user one
        #[arg(long)]
        system: bool,
        /// Start the user unit at boot without login (loginctl
        /// enable-linger)
        #[arg(long)]
        linger: bool,
        /// Override the ExecStart command (default: this binary + ' serve')
        #[arg(long)]
        exec: Option<String>,
        /// Environment file path (default: ~/.config/peregrin/env)
        #[arg(long)]
        env_file: Option<PathBuf>,
    },
    /// Stop, disable and remove the peregrin service
    Uninstall {
        /// Manage the hub unit (peregrin-hub) instead of the node unit
        #[arg(long)]
        hub: bool,
        /// Install/manage the system-level unit instead of the per-user one
        #[arg(long)]
        system: bool,
        /// Also delete the env file (kept by default)
        #[arg(long)]
        purge: bool,
    },
    /// Start the service now and at boot/login (keeps the unit file —
    /// the counterpart of `disable`, not a reinstall)
    Enable {
        /// Manage the hub unit (peregrin-hub) instead of the node unit
        #[arg(long)]
        hub: bool,
        /// Install/manage the system-level unit instead of the per-user one
        #[arg(long)]
        system: bool,
    },
    /// Stop the service now and keep it stopped across boots/logins —
    /// the unit file stays so `enable` brings it back
    Disable {
        /// Manage the hub unit (peregrin-hub) instead of the node unit
        #[arg(long)]
        hub: bool,
        /// Install/manage the system-level unit instead of the per-user one
        #[arg(long)]
        system: bool,
    },
    /// Show whether the peregrin service is installed, enabled and running
    Status {
        /// Manage the hub unit (peregrin-hub) instead of the node unit
        #[arg(long)]
        hub: bool,
        /// Install/manage the system-level unit instead of the per-user one
        #[arg(long)]
        system: bool,
    },
    /// Restart the peregrin service
    Restart {
        /// Manage the hub unit (peregrin-hub) instead of the node unit
        #[arg(long)]
        hub: bool,
        /// Install/manage the system-level unit instead of the per-user one
        #[arg(long)]
        system: bool,
    },
    /// Print (or follow, -f) the peregrin service log
    Logs {
        /// Manage the hub unit (peregrin-hub) instead of the node unit
        #[arg(long)]
        hub: bool,
        /// Install/manage the system-level unit instead of the per-user one
        #[arg(long)]
        system: bool,
        /// Follow the log stream (journalctl -f / tail -f)
        #[arg(short = 'f', long = "f")]
        follow: bool,
    },
}

// --- distribution (ADR-0079) ---------------------------------------------------

/// Fetch-cache management — `~/.cache/peregrin/fetch/<key>` entries.
#[derive(Subcommand)]
enum CacheCommands {
    /// Evict cache entries — by age (--days, default 30) or --all
    Clean {
        /// Evict the entire fetch cache
        #[arg(long)]
        all: bool,
        /// Evict entries older than this many days (default: 30)
        #[arg(long)]
        days: Option<u64>,
    },
}

/// `peregrin import <source>` — foreign config → peregrin node-config
/// workflow conversion (ADR-0081).
#[derive(Subcommand)]
enum ImportCommands {
    /// Convert a bupha.toml/bupha.yaml into a node-config workflow
    /// (ingress github-webhook node + per-repo/job gate pipelines)
    Bupha {
        /// Path to the bupha config (TOML or YAML — extension picks the parser)
        path: PathBuf,
        /// Write the emitted workflow to this file (default: stdout)
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

/// Bundle authoring verbs.
#[derive(Subcommand)]
enum BundleCommands {
    /// Pack a bundle dir (manifest + payload) into an installable .tgz
    Pack {
        /// Bundle directory (must carry tamias.yaml/.yml/.toml)
        dir: PathBuf,
        /// Output file (default: <name>[-<version>].tgz)
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

// --- ops (ADR-0079 provision/teardown + observability) ---------------------------

/// Deployment verbs — the parts of `create` that hit the outside
/// world (hook registration against the forge API).
#[derive(Subcommand)]
enum DeployCommands {
    /// Register/update each watched repo's push webhook via the
    /// forge REST API (token + per-repo hook secrets resolve by NAME
    /// through env → keyring → ~/.config/peregrin/env)
    Webhook {
        /// Node config to read (default: ./peregrin.* then
        /// ~/.config/peregrin/peregrin.*)
        #[arg(long)]
        config: Option<PathBuf>,
        /// Public URL the hooks point at (default:
        /// config.public_url); the hook lands at <url><hook-path>
        #[arg(long)]
        url: Option<String>,
        /// Secret NAME holding the forge token (default:
        /// config.token_env, else GITHUB_TOKEN)
        #[arg(long)]
        token_env: Option<String>,
    },
}

/// Teardown counterparts of `deploy`/`create`.
#[derive(Subcommand)]
enum DestroyCommands {
    /// Delete the hooks `deploy webhook` registered — exact URL match
    Webhook {
        /// Only this watched repo (or --all)
        #[arg(long, required_unless_present = "all", conflicts_with = "all")]
        repo: Option<String>,
        /// Every repo the node config's github-webhook nodes watch
        #[arg(long, required_unless_present = "repo", conflicts_with = "repo")]
        all: bool,
        /// Node config to read (default: ./peregrin.* then
        /// ~/.config/peregrin/peregrin.*)
        #[arg(long)]
        config: Option<PathBuf>,
        /// Public URL the hooks point at (default: config.public_url)
        #[arg(long)]
        url: Option<String>,
        /// Secret NAME holding the forge token
        #[arg(long)]
        token_env: Option<String>,
    },
    /// Disable + remove a campaign unit (<name>.service)
    Service {
        /// Unit name — the `name` (or --unit) `create` used
        name: String,
    },
    /// Clean leftover run worktrees (tracked ones are destroyed,
    /// orphans reported)
    Worktrees {
        /// Worktree base dir (default: config.worktree_base, else
        /// ~/.local/share/peregrin/worktrees)
        #[arg(long)]
        base: Option<PathBuf>,
        /// Node config to read the base from
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

// --- root ----------------------------------------------------------------------

/// Drive a peregrin node over its API, or convert sessions between the
/// Devin, Cline, Claude and Cursor stores
#[derive(Parser)]
#[command(name = "peregrin", version = PEREGRIN_VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // Node ops — hit a running node's REST API (--node/--token,
    // PEREGRIN_NODE_URL/PEREGRIN_TOKEN).
    /// GET /api/health — is the node up and its store readable
    Health {
        #[command(flatten)]
        args: NodeArgs,
    },
    /// GET /api/node — the node's identity, agents and capabilities
    Node {
        #[command(flatten)]
        args: NodeArgs,
    },
    /// GET /api/agents — the agent runtimes registered on the node
    Agents {
        #[command(flatten)]
        args: NodeArgs,
    },
    /// GET /api/user — the OS user the node runs as
    User {
        #[command(flatten)]
        args: NodeArgs,
    },
    /// GET /api/fs — list subdirectories of a path on the node
    Fs {
        /// Absolute directory path on the node
        path: String,
        #[command(flatten)]
        args: NodeArgs,
    },
    /// GET /api/events — stream the node event feed
    /// (session/meta/project/heartbeat) to stdout
    Events {
        #[command(flatten)]
        args: TargetArgs,
    },
    /// POST /api/pair — exchange a `peregrin pair` code for a long-lived
    /// bearer token
    Redeem {
        /// One-time code printed by `peregrin pair` on the node
        code: String,
        /// Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else
        /// http://127.0.0.1:18787)
        #[arg(long, default_value_t = api::default_node_url())]
        node: String,
    },
    /// POST /api/sessions/:id/prompt — send one turn to an attached session
    Prompt {
        #[arg(help = SESSION_ID_HELP)]
        session_id: String,
        /// The prompt text to send
        text: String,
        #[command(flatten)]
        agent: AgentQuery,
        #[command(flatten)]
        args: TargetArgs,
    },
    /// Node ops against /api/sessions — run against a live peregrin node
    #[command(subcommand)]
    Sessions(SessionsCommands),
    /// Campaign ops — load and run campaign yamls on this machine's
    /// campaign plane (the `handl` verbs: run, validate)
    #[command(subcommand)]
    Campaign(CampaignCommands),
    /// Node-local project grouping (/api/projects)
    #[command(subcommand)]
    Projects(ProjectsCommands),
    /// Config ops — local agent config IR (list/export/import/install/
    /// diff) plus the node's server-side config (get/set)
    #[command(subcommand)]
    Config(ConfigCommands),
    /// Web-push subscription management (/api/push/*)
    #[command(subcommand)]
    Push(PushCommands),
    /// Print a one-time pairing code — the UI exchanges it via
    /// POST /api/pair
    Pair {
        /// PEREGRIN_HOME of the node to pair with
        #[arg(long, default_value_os_t = default_peregrin_home())]
        home: PathBuf,
        /// The node's URL, printed for the pairing UI
        #[arg(long, default_value_t = default_pair_url())]
        url: String,
    },
    /// Serve the peregrin node — the `/api/*` surface (PEREGRIN_* env
    /// configures it)
    Serve,
    /// Manage the peregrin node as an OS service (systemd user unit /
    /// launchd plist)
    Service {
        /// Manage this unit instead of `peregrin`/`peregrin-hub` —
        /// e.g. a unit `create --template` wrote (wins over --hub)
        #[arg(long, global = true)]
        unit: Option<String>,
        #[command(subcommand)]
        cmd: ServiceCommands,
    },
    // Distribution verbs (ADR-0079) — local store/cache ops, no node.
    /// Install a bundle — ./path, git:<url>[//subdir][#rev], or an
    /// http(s) .tgz/.tar.gz URL
    Install {
        /// The bundle ref
        reference: String,
    },
    /// Drop an installed bundle's store entry (the fetch cache is
    /// untouched)
    Remove {
        /// Bundle name (or name@version when several are installed)
        name: String,
    },
    /// List installed bundles — name@version, resolved sha, source ref
    List,
    /// Show a bundle's manifest, provenance and size
    Info {
        /// Bundle name (or name@version)
        name: String,
    },
    /// Re-resolve a bundle's recorded source ref — the upgrade path
    /// for moving refs
    Update {
        /// Bundle name (omit with --all)
        name: Option<String>,
        /// Update every installed bundle
        #[arg(long, required_unless_present = "name", conflicts_with = "name")]
        all: bool,
    },
    /// Retarget a bundle's recorded source ref (refetches when the
    /// resolved digest differs)
    Pin {
        /// Bundle name (or name@version)
        name: String,
        /// The new source ref
        reference: String,
    },
    /// Digest-check a bundle's store contents against its recorded sha
    Verify {
        /// Bundle name (or name@version)
        name: String,
    },
    /// Fetch-cache management
    #[command(subcommand)]
    Cache(CacheCommands),
    /// Bundle authoring
    #[command(subcommand)]
    Bundle(BundleCommands),
    /// Foreign config → peregrin node-config workflow conversion
    /// (ADR-0081 bupha importer)
    #[command(subcommand)]
    Import(ImportCommands),
    // Ops verbs (ADR-0079 provision/teardown) — local store + unit +
    // forge API ops, no node.
    /// Materialize a node config + systemd unit from an installed
    /// bundle — the parameterized install (writes files only; the
    /// operator enables the service separately)
    Create {
        /// Installed bundle name (or name@version)
        #[arg(long)]
        template: String,
        /// Instance name (default: the bundle name) — drives the
        /// config file <name>.yaml and unit <name>.service
        #[arg(long)]
        name: Option<String>,
        /// Merge KEY=VALUE into the campaign's `config:` — repeatable
        #[arg(long = "set")]
        sets: Vec<String>,
        /// Watch owner/name[:SECRET_ENV] via a github-webhook ingress
        /// node — repeatable
        #[arg(long = "repo")]
        repos: Vec<String>,
        /// Write the node config here (default:
        /// ~/.config/peregrin/<name>.yaml)
        #[arg(long)]
        config: Option<PathBuf>,
        /// Unit name (default: <name>)
        #[arg(long)]
        unit: Option<String>,
    },
    /// Deployment verbs (register the configured webhooks)
    #[command(subcommand)]
    Deploy(DeployCommands),
    /// Teardown verbs (hooks, units, worktrees)
    #[command(subcommand)]
    Destroy(DestroyCommands),
    /// Recent runs — per-push records read back from the durable
    /// JSONL history (newest first)
    Runs {
        /// The event log to read (default: $PEREGRIN_HOME/history.jsonl)
        #[arg(long)]
        history: Option<PathBuf>,
        /// Filter to one repo ("owner/name" also matches its job keys)
        #[arg(long)]
        repo: Option<String>,
        /// Limit to the newest N runs
        #[arg(short = 'n', long)]
        limit: Option<usize>,
    },
    /// Raw event log — the last N lines of the JSONL history
    Log {
        /// Lines to show
        #[arg(short, long, default_value = "50")]
        n: usize,
        /// The event log to read (default: $PEREGRIN_HOME/history.jsonl)
        #[arg(long)]
        history: Option<PathBuf>,
    },
    /// Why a sha did/didn't run — the run's events + outcome, or the
    /// leftover worktree it died in
    Explain {
        /// The pushed sha (any prefix) or an event id
        target: String,
        /// The event log to read (default: $PEREGRIN_HOME/history.jsonl)
        #[arg(long)]
        history: Option<PathBuf>,
        /// Node config to read the worktree base from
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Check what the node config actually needs on this box —
    /// config loads, dirs exist, secret NAMES resolve, tools present
    Doctor {
        /// Node config to check (default: ./peregrin.* then
        /// ~/.config/peregrin/peregrin.*)
        #[arg(long)]
        config: Option<PathBuf>,
        /// Machine-readable JSON output — {ok, failures, checks}
        #[arg(long)]
        json: bool,
    },
    /// Print the peregrin version stamp
    Version,
}

/// One runtime for the whole command — store drivers are sync under the
/// async trait; `serve` upgrades to multi-thread.
fn runtime() -> Result<tokio::runtime::Runtime, CliError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError(format!("failed to start runtime: {e}")))
}

fn run_sessions(cmd: &SessionsCommands) -> Result<(), CliError> {
    match cmd {
        SessionsCommands::List { locks, args } => node_ops::sessions_list(*locks, args),
        SessionsCommands::Create {
            cwd,
            agent,
            title,
            model,
            fallback,
            args,
        } => node_ops::sessions_create(
            &cwd.to_string_lossy(),
            agent.as_deref(),
            title.as_deref(),
            model.as_deref(),
            fallback,
            args,
        ),
        SessionsCommands::Attach {
            session_id,
            takeover,
            model,
            fallback,
            agent,
            args,
        } => node_ops::sessions_attach(
            session_id,
            *takeover,
            model.as_deref(),
            fallback,
            agent.agent.as_deref(),
            args,
        ),
        SessionsCommands::Prompt {
            session_id,
            text,
            agent,
            args,
        } => node_ops::sessions_prompt(session_id, text, agent.agent.as_deref(), args),
        SessionsCommands::Cancel {
            session_id,
            agent,
            args,
        } => node_ops::sessions_cancel(session_id, agent.agent.as_deref(), args),
        SessionsCommands::Permission {
            session_id,
            request,
            option,
            agent,
            args,
        } => node_ops::sessions_permission(
            session_id,
            request,
            option.as_deref(),
            agent.agent.as_deref(),
            args,
        ),
        SessionsCommands::History {
            session_id,
            limit,
            before,
            agent,
            args,
        } => node_ops::sessions_history(session_id, *limit, *before, agent.agent.as_deref(), args),
        SessionsCommands::Checkpoints {
            session_id,
            agent,
            args,
        } => node_ops::sessions_checkpoints(session_id, agent.agent.as_deref(), args),
        SessionsCommands::Export {
            session_id,
            out,
            agent,
            args,
        } => node_ops::sessions_export(session_id, out.as_deref(), agent.agent.as_deref(), args),
        SessionsCommands::Stream {
            session_id,
            agent,
            args,
        } => node_ops::sessions_stream(session_id, agent.agent.as_deref(), args),
        SessionsCommands::Meta {
            session_id,
            title,
            pin,
            unpin,
            archive,
            unarchive,
            project,
            model,
            clear_model,
            agent,
            args,
        } => node_ops::sessions_meta(
            session_id,
            title.as_deref(),
            *pin,
            *unpin,
            *archive,
            *unarchive,
            project,
            model.as_deref(),
            *clear_model,
            agent.agent.as_deref(),
            args,
        ),
        SessionsCommands::Rename {
            session_id,
            title,
            agent,
            args,
        } => node_ops::sessions_rename(session_id, title, agent.agent.as_deref(), args),
        SessionsCommands::Delete {
            session_id,
            agent,
            args,
        } => node_ops::sessions_delete(session_id, agent.agent.as_deref(), args),
        SessionsCommands::Convert {
            session_id,
            to,
            args,
        } => node_ops::sessions_convert(session_id, to.as_str(), args),
        SessionsCommands::Import {
            path,
            to,
            cwd,
            title,
            model,
            args,
        } => node_ops::sessions_import(
            &path.to_string_lossy(),
            to.as_str(),
            cwd.as_deref(),
            title.as_deref(),
            model.as_deref(),
            args,
        ),
        SessionsCommands::Resume {
            session_id,
            to,
            source,
            source_token,
            from_agent,
            cwd,
            title,
            model,
            args,
        } => node_ops::sessions_resume(
            session_id,
            to.as_str(),
            source,
            source_token.as_deref(),
            from_agent.as_deref(),
            cwd.as_deref(),
            title.as_deref(),
            model.as_deref(),
            args,
        ),
        SessionsCommands::Restore {
            session_id,
            confirm,
            path,
            tool_call_id,
            checkpoint,
            paths,
            agent,
            args,
        } => node_ops::sessions_restore(
            session_id,
            *confirm,
            path.as_deref(),
            tool_call_id.as_deref(),
            checkpoint.as_deref(),
            paths,
            agent.agent.as_deref(),
            args,
        ),
        SessionsCommands::Rewind {
            session_id,
            confirm,
            node_id,
            turns,
            checkpoint,
            agent,
            args,
        } => node_ops::sessions_rewind(
            session_id,
            *confirm,
            *node_id,
            *turns,
            checkpoint.as_deref(),
            agent.agent.as_deref(),
            args,
        ),
    }
}

fn run_projects(cmd: &ProjectsCommands) -> Result<(), CliError> {
    match cmd {
        ProjectsCommands::List { args } => node_ops::projects_list(args),
        ProjectsCommands::Create { name, args } | ProjectsCommands::Init { name, args } => {
            node_ops::projects_create(name, args)
        }
        ProjectsCommands::Rename {
            project_id,
            name,
            args,
        } => node_ops::projects_rename(project_id, name, args),
        ProjectsCommands::Delete { project_id, args } => {
            node_ops::projects_delete(project_id, args)
        }
    }
}

fn run_campaign(cmd: &CampaignCommands) -> Result<(), CliError> {
    match cmd {
        CampaignCommands::Run {
            workflow,
            dry_run,
            replay,
            history,
        } => campaign::run(workflow, *dry_run, replay.as_deref(), history.as_deref()).map(|_| ()),
        CampaignCommands::Validate { workflow } => campaign::validate(workflow).map(|_| ()),
    }
}

fn run_config(cmd: &ConfigCommands) -> Result<(), CliError> {
    match cmd {
        ConfigCommands::Get { args } => node_ops::config_get(args),
        ConfigCommands::Set { key, value, args } => node_ops::config_set(key, value, args),
    }
}

fn run_push(cmd: &PushCommands) -> Result<(), CliError> {
    match cmd {
        PushCommands::Vapid { args } => node_ops::push_vapid(args),
        PushCommands::Subscribe { subscription, args } => {
            node_ops::push_subscribe(subscription, args)
        }
        PushCommands::Unsubscribe { endpoint, args } => node_ops::push_unsubscribe(endpoint, args),
    }
}

fn run_service(cmd: &ServiceCommands, unit: Option<&str>) -> Result<(), CliError> {
    match cmd {
        ServiceCommands::Install {
            hub,
            system,
            linger,
            exec,
            env_file,
        } => service::service_install(
            *hub,
            *system,
            *linger,
            exec.as_deref(),
            env_file.as_deref(),
            unit,
        ),
        ServiceCommands::Uninstall { hub, system, purge } => {
            service::service_uninstall(*hub, *system, *purge, unit)
        }
        ServiceCommands::Enable { hub, system } => service::service_enable(*hub, *system, unit),
        ServiceCommands::Disable { hub, system } => service::service_disable(*hub, *system, unit),
        ServiceCommands::Status { hub, system } => service::service_status(*hub, *system, unit),
        ServiceCommands::Restart { hub, system } => service::service_restart(*hub, *system, unit),
        ServiceCommands::Logs {
            hub,
            system,
            follow,
        } => service::service_logs(*hub, *system, *follow, unit),
    }
}

/// Coverage probe — `serve` binds a port and blocks in `axum::serve`
/// until SIGTERM; an in-process call would hang the test binary. The
/// real body is compiled out under `cfg(coverage)`; spawned-binary
/// coverage comes from tests that launch `peregrin serve` for real.
#[cfg(coverage)]
fn serve() -> Result<(), CliError> {
    let _telemetry = peregrin_telemetry::init();
    let _ = peregrin_http::Env::parse().map_err(|e| CliError(e.to_string()))?;
    Err(CliError(
        "serve is excluded under coverage — it blocks forever".into(),
    ))
}

#[cfg(not(coverage))]
fn serve() -> Result<(), CliError> {
    // fmt + RUST_LOG as always; OTLP span export layered on when
    // PEREGRIN_OTEL_ENDPOINT / OTEL_EXPORTER_OTLP_* is set. The guard's
    // Drop flushes the provider when `block_on` returns.
    let _telemetry = peregrin_telemetry::init();
    let env = peregrin_http::Env::parse().map_err(|e| CliError(e.to_string()))?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError(format!("failed to start runtime: {e}")))?
        .block_on(peregrin_node::serve(&env))
        .map_err(|e| CliError(e.to_string()))
}

/// The default store/cache roots — extracted so a test can swap the
/// home dirs via env before dispatching.
fn dist_env() -> distribution::DistEnv {
    distribution::DistEnv::default()
}

/// The ops roots (bundle store, config dir, unit dir, PEREGRIN_HOME).
fn ops_env() -> ops::OpsEnv {
    ops::OpsEnv::default()
}

fn dispatch(cli: &Cli, _rt: &tokio::runtime::Runtime) -> Result<(), CliError> {
    match &cli.command {
        Commands::Health { args } => node_ops::health(args),
        Commands::Node { args } => node_ops::node(args),
        Commands::Agents { args } => node_ops::agents(args),
        Commands::User { args } => node_ops::user(args),
        Commands::Fs { path, args } => node_ops::fs(path, args),
        Commands::Events { args } => node_ops::events(args),
        Commands::Redeem { code, node } => node_ops::redeem(
            code,
            &TargetArgs {
                node: node.clone(),
                token: None,
            },
        ),
        Commands::Prompt {
            session_id,
            text,
            agent,
            args,
        } => node_ops::sessions_prompt(session_id, text, agent.agent.as_deref(), args),
        Commands::Sessions(cmd) => run_sessions(cmd),
        Commands::Campaign(cmd) => run_campaign(cmd),
        Commands::Projects(cmd) => run_projects(cmd),
        Commands::Config(cmd) => run_config(cmd),
        Commands::Push(cmd) => run_push(cmd),
        Commands::Pair { home, url } => pair::pair(home, url),
        Commands::Service { unit, cmd } => run_service(cmd, unit.as_deref()),
        Commands::Install { reference } => {
            distribution::install(&dist_env(), reference).map(|_| ())
        }
        Commands::Remove { name } => distribution::remove(&dist_env(), name).map(|_| ()),
        Commands::List => distribution::list(&dist_env()).map(|_| ()),
        Commands::Info { name } => distribution::info(&dist_env(), name).map(|_| ()),
        Commands::Update { name, all } => {
            distribution::update(&dist_env(), name.as_deref(), *all).map(|_| ())
        }
        Commands::Pin { name, reference } => {
            distribution::pin(&dist_env(), name, reference).map(|_| ())
        }
        Commands::Verify { name } => distribution::verify(&dist_env(), name).map(|_| ()),
        Commands::Cache(CacheCommands::Clean { all, days }) => {
            distribution::cache_clean(&dist_env(), *all, *days).map(|_| ())
        }
        Commands::Bundle(BundleCommands::Pack { dir, out }) => {
            distribution::bundle_pack(dir, out.as_deref()).map(|_| ())
        }
        Commands::Import(ImportCommands::Bupha { path, out }) => {
            import::import_bupha(path, out.as_deref()).map(|_| ())
        }
        Commands::Create {
            template,
            name,
            sets,
            repos,
            config,
            unit,
        } => ops::create(
            &ops_env(),
            template,
            name.as_deref(),
            sets,
            repos,
            config.as_deref(),
            unit.as_deref(),
        )
        .map(|_| ()),
        Commands::Deploy(DeployCommands::Webhook {
            config,
            url,
            token_env,
        }) => ops::deploy_webhook(
            &ops_env(),
            config.as_deref(),
            url.as_deref(),
            token_env.as_deref(),
        )
        .map(|_| ()),
        Commands::Destroy(DestroyCommands::Webhook {
            repo,
            all,
            config,
            url,
            token_env,
        }) => ops::destroy_webhook(
            &ops_env(),
            config.as_deref(),
            url.as_deref(),
            token_env.as_deref(),
            repo.as_deref(),
            *all,
        )
        .map(|_| ()),
        Commands::Destroy(DestroyCommands::Service { name }) => {
            ops::destroy_service(&ops_env(), name).map(|_| ())
        }
        Commands::Destroy(DestroyCommands::Worktrees { base, config }) => {
            ops::destroy_worktrees(&ops_env(), config.as_deref(), base.as_deref()).map(|_| ())
        }
        Commands::Runs {
            history,
            repo,
            limit,
        } => ops::runs(&ops_env(), history.as_deref(), repo.as_deref(), *limit).map(|_| ()),
        Commands::Log { n, history } => ops::log(&ops_env(), history.as_deref(), *n).map(|_| ()),
        Commands::Explain {
            target,
            history,
            config,
        } => ops::explain(&ops_env(), history.as_deref(), config.as_deref(), target).map(|_| ()),
        Commands::Doctor { config, json } => {
            let report = ops::doctor(&ops_env(), config.as_deref(), *json)?;
            if report.failures > 0 {
                return Err(CliError(format!("{} problem(s)", report.failures)));
            }
            Ok(())
        }
        Commands::Version => {
            println!("{PEREGRIN_VERSION}");
            Ok(())
        }
        Commands::Serve => serve(),
    }
}

fn exit_code(result: Result<(), CliError>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    exit_code(runtime().and_then(|rt| dispatch(&cli, &rt)))
}

#[cfg(test)]
mod tests {
    //! Dispatch coverage — every `Commands`/`SessionsCommands`/… arm is
    //! reached by parsing argv and running `dispatch` against either a
    //! dead node (fast Err) or the canned node in `tests/node_ops.rs`.
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn rt() -> tokio::runtime::Runtime {
        runtime().unwrap()
    }

    fn cli(argv: &[&str]) -> Cli {
        Cli::try_parse_from(argv).unwrap()
    }

    #[test]
    fn parse_and_dispatch_safe_verbs() {
        let rt = rt();
        // Dead node — every dispatch arm runs and errs fast (no
        // network), which is all the arm needs.
        let dead = "http://127.0.0.1:1";
        for argv in [
            vec!["peregrin", "health", "--node", dead],
            vec!["peregrin", "node", "--node", dead],
            vec!["peregrin", "agents", "--node", dead],
            vec!["peregrin", "user", "--node", dead],
            vec!["peregrin", "fs", "/tmp", "--node", dead],
            vec!["peregrin", "redeem", "code", "--node", dead],
            vec!["peregrin", "prompt", "s1", "hi", "--node", dead],
            vec!["peregrin", "sessions", "list", "--node", dead],
            vec!["peregrin", "sessions", "create", "--node", dead],
            vec!["peregrin", "sessions", "attach", "s1", "--node", dead],
            vec!["peregrin", "sessions", "prompt", "s1", "hi", "--node", dead],
            vec!["peregrin", "sessions", "cancel", "s1", "--node", dead],
            vec![
                "peregrin",
                "sessions",
                "permission",
                "s1",
                "--request",
                "r1",
                "--node",
                dead,
            ],
            vec!["peregrin", "sessions", "history", "s1", "--node", dead],
            vec!["peregrin", "sessions", "checkpoints", "s1", "--node", dead],
            vec!["peregrin", "sessions", "export", "s1", "--node", dead],
            vec!["peregrin", "sessions", "stream", "s1", "--node", dead],
            vec![
                "peregrin", "sessions", "meta", "s1", "--title", "t", "--node", dead,
            ],
            vec!["peregrin", "sessions", "rename", "s1", "t", "--node", dead],
            vec!["peregrin", "sessions", "delete", "s1", "--node", dead],
            vec![
                "peregrin", "sessions", "convert", "s1", "--to", "cline", "--node", dead,
            ],
            vec![
                "peregrin",
                "sessions",
                "restore",
                "s1",
                "--confirm",
                "--path",
                "a",
                "--node",
                dead,
            ],
            vec![
                "peregrin",
                "sessions",
                "rewind",
                "s1",
                "--confirm",
                "--node-id",
                "1",
                "--node",
                dead,
            ],
            // Campaign verbs — load fails fast on a missing file.
            vec!["peregrin", "campaign", "run", "/nonexistent.yaml"],
            vec!["peregrin", "campaign", "validate", "/nonexistent.yaml"],
            vec![
                "peregrin",
                "campaign",
                "run",
                "/nonexistent.yaml",
                "--dry-run",
                "--replay",
                "/nonexistent.jsonl",
            ],
            vec!["peregrin", "projects", "list", "--node", dead],
            vec!["peregrin", "projects", "create", "n", "--node", dead],
            vec!["peregrin", "projects", "init", "n", "--node", dead],
            vec!["peregrin", "projects", "rename", "p1", "n", "--node", dead],
            vec!["peregrin", "projects", "delete", "p1", "--node", dead],
            vec!["peregrin", "config", "get", "--node", dead],
            vec!["peregrin", "config", "set", "k", "1", "--node", dead],
            vec!["peregrin", "push", "vapid", "--node", dead],
            vec!["peregrin", "push", "subscribe", "{}", "--node", dead],
            vec!["peregrin", "push", "unsubscribe", "e", "--node", dead],
            vec!["peregrin", "events", "--node", dead],
            vec![
                "peregrin",
                "sessions",
                "import",
                "/nonexistent.json",
                "--to",
                "devin",
                "--node",
                dead,
            ],
            vec![
                "peregrin", "sessions", "resume", "s1", "--to", "devin", "--source", dead,
                "--node", dead,
            ],
            // Distribution verbs — every arm errs fast on a bad
            // ref/name before touching the real store, except `list`
            // which is read-only.
            vec!["peregrin", "install", "/definitely/not/a/path"],
            vec!["peregrin", "install", "not-a-ref"],
            vec!["peregrin", "remove", "ghost"],
            vec!["peregrin", "list"],
            vec!["peregrin", "info", "ghost"],
            vec!["peregrin", "update", "ghost"],
            vec!["peregrin", "pin", "ghost", "./x"],
            vec!["peregrin", "verify", "ghost"],
            // `cache clean` alone is a no-op on a cache this feature
            // just created (30-day eviction); --all/--days variants
            // are covered in tests/distribution.rs instead — running
            // them here would wipe a real ~/.cache/peregrin.
            vec!["peregrin", "cache", "clean"],
            vec!["peregrin", "bundle", "pack", "/definitely/not/a/dir"],
            vec!["peregrin", "import", "bupha", "/definitely/not/a/file"],
            // Ops verbs — every arm errs fast on a missing
            // bundle/config before touching the network or the
            // filesystem; the read verbs are safe on any home.
            vec!["peregrin", "create", "--template", "ghost"],
            vec![
                "peregrin",
                "deploy",
                "webhook",
                "--config",
                "/nonexistent.yaml",
            ],
            vec![
                "peregrin",
                "destroy",
                "webhook",
                "--repo",
                "a/b",
                "--config",
                "/nonexistent.yaml",
            ],
            // `destroy service` must target a name NO other test
            // installs — it really removes ~/.config/systemd/user/
            // <name>.service on this host.
            vec!["peregrin", "destroy", "service", "peregrin-ops-covtest"],
            vec![
                "peregrin",
                "destroy",
                "worktrees",
                "--base",
                "/definitely/not/a/base",
            ],
            vec!["peregrin", "runs"],
            vec!["peregrin", "runs", "--repo", "a/b", "-n", "1"],
            vec!["peregrin", "log", "-n", "5"],
            vec!["peregrin", "explain", "feedbeef"],
            vec!["peregrin", "doctor"],
            vec!["peregrin", "doctor", "--json"],
            vec![
                "peregrin",
                "import",
                "bupha",
                "/definitely/not/a/file",
                "--out",
                "/tmp/x.yaml",
            ],
        ] {
            let cli = cli(&argv);
            let _ = dispatch(&cli, &rt);
        }
        // Service verbs + `serve`: under `cfg(coverage)` they're compiled
        // to fast-Err probes, which is the *only* build where dispatching
        // them in-process is safe — outside coverage `service install`
        // writes a real unit and runs `systemctl --user enable --now`,
        // and `serve` binds :18787 and blocks forever.
        #[cfg(coverage)]
        for argv in [
            vec!["peregrin", "service", "install"],
            vec!["peregrin", "service", "uninstall"],
            vec!["peregrin", "service", "enable"],
            vec!["peregrin", "service", "disable"],
            vec!["peregrin", "service", "status"],
            vec!["peregrin", "service", "restart"],
            vec!["peregrin", "service", "logs"],
            vec![
                "peregrin", "service", "install", "--hub", "--system", "--linger",
            ],
            vec!["peregrin", "service", "--unit", "covtest", "status"],
            vec!["peregrin", "service", "enable", "--unit", "covtest"],
            vec!["peregrin", "serve"],
        ] {
            let cli = cli(&argv);
            let _ = dispatch(&cli, &rt);
        }
        // Pure arm: version prints.
        let cli = cli(&["peregrin", "version"]);
        dispatch(&cli, &rt).unwrap();
        // Pair writes into a tempdir home.
        let dir = tempfile::TempDir::new().unwrap();
        let home = dir.path().join("home");
        let cli = self::cli(&["peregrin", "pair", "--home", home.to_str().unwrap()]);
        dispatch(&cli, &rt).unwrap();
        assert!(home.join("pair-code").exists());
        // ImportTo::as_str both arms.
        assert_eq!(ImportTo::Cline.as_str(), "cline");
        assert_eq!(ImportTo::Devin.as_str(), "devin");
        // Helpers.
        let _ = default_peregrin_home();
        let _ = default_pair_url();
        assert!(!PEREGRIN_VERSION.is_empty());
    }

    #[test]
    fn exit_code_both_arms() {
        assert_eq!(exit_code(Ok(())), ExitCode::SUCCESS);
        assert_eq!(exit_code(Err(CliError("boom".into()))), ExitCode::FAILURE);
    }

    #[test]
    fn clap_shape() {
        // The derive surface itself: long help renders without panic.
        let _ = Cli::try_parse_from(["peregrin", "--help"]);
        let _ = Cli::try_parse_from(["peregrin", "sessions", "--help"]);
        let _ = Cli::try_parse_from(["peregrin", "campaign", "--help"]);
        let _ = Cli::try_parse_from(["peregrin", "service", "--help"]);
        let _ = Cli::try_parse_from(["peregrin", "bogus"]);
    }
}
