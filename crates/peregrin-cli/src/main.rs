//! peregrin — the CLI. Local store verbs over the
//! Devin/Cline/Claude/Cursor stores, node ops against a running node's
//! `/api/*` surface, agent config IR verbs, pairing, `serve` (the node
//! itself, shared with `peregrin-node`), and `service` management
//! (systemd/launchd).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};

use peregrin_cli::node_ops::{NodeArgs, TargetArgs};
use peregrin_cli::{CliError, PEREGRIN_VERSION, api, node_ops, pair, service};

fn default_peregrin_home() -> PathBuf {
    peregrin_node::peregrin_home()
}

fn default_pair_url() -> String {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8787".into());
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
        /// http://127.0.0.1:8787)
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
    #[command(subcommand)]
    Service(ServiceCommands),
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

fn run_service(cmd: &ServiceCommands) -> Result<(), CliError> {
    match cmd {
        ServiceCommands::Install {
            hub,
            system,
            linger,
            exec,
            env_file,
        } => service::service_install(*hub, *system, *linger, exec.as_deref(), env_file.as_deref()),
        ServiceCommands::Uninstall { hub, system, purge } => {
            service::service_uninstall(*hub, *system, *purge)
        }
        ServiceCommands::Status { hub, system } => service::service_status(*hub, *system),
        ServiceCommands::Restart { hub, system } => service::service_restart(*hub, *system),
        ServiceCommands::Logs {
            hub,
            system,
            follow,
        } => service::service_logs(*hub, *system, *follow),
    }
}

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
        Commands::Projects(cmd) => run_projects(cmd),
        Commands::Config(cmd) => run_config(cmd),
        Commands::Push(cmd) => run_push(cmd),
        Commands::Pair { home, url } => pair::pair(home, url),
        Commands::Service(cmd) => run_service(cmd),
        Commands::Version => {
            println!("{PEREGRIN_VERSION}");
            Ok(())
        }
        Commands::Serve => serve(),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match runtime().and_then(|rt| dispatch(&cli, &rt)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
