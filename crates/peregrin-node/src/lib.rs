//! peregrin-node — the machine daemon. Builds the local session plane
//! (`sepia-session::local`), mounts the `/api/*` surface, and shuts
//! the plane down on SIGINT/SIGTERM. Orchestration/policy layers sit
//! above this: campaigns, multi-node arbitration, the hub.

use std::path::PathBuf;
use std::sync::Arc;

use peregrin_http::AppState;
use peregrin_http::env::Env;
use peregrin_http::routes::sessions::ImportTarget;
use peregrin_session::SessionPlane;
use sepia_core::Session;
use sepia_session::{ControlError, ControlErrorCode, Paths, base_options, build};

/// Default `PEREGRIN_HOME`: `PEREGRIN_HOME` env or
/// `~/.local/share/peregrin`. (The session plane's own `SEPIA_HOME`
/// default is unrelated — node paths are passed explicitly.)
pub fn peregrin_home() -> PathBuf {
    std::env::var_os("PEREGRIN_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/peregrin"))
        })
        .unwrap_or_else(|| PathBuf::from(".peregrin"))
}

fn control_err(e: impl std::fmt::Display) -> ControlError {
    ControlError {
        code: ControlErrorCode::Internal,
        message: e.to_string(),
        cause: Some(e.to_string()),
    }
}

/// Build + serve the node — the whole `peregrin-node` boot path, shared
/// with `peregrin serve` so both binaries behave identically. Call on
/// a tokio runtime; returns when the listener fails or after graceful
/// shutdown.
///
/// # Errors
/// `std::io::Error` on plane build, bind, or serve failure.
pub async fn serve(env: &Env) -> std::io::Result<()> {
    // `PEREGRIN_SESSION_PLANE` selects the wire mode — tests can't set
    // process env under `forbid(unsafe_code)`, so the spawn-a-`sepia
    // serve`-child arm is compiled out of coverage builds (it runs in
    // the spawned-node e2e instead).
    #[cfg(not(coverage))]
    let service_mode = std::env::var("PEREGRIN_SESSION_PLANE").as_deref() == Ok("service");
    #[cfg(not(coverage))]
    if service_mode {
        let spec = peregrin_session::Spec {
            binary: peregrin_session::sepia_binary().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "sepia binary not found — set PEREGRIN_SEPIA or install sepia",
                )
            })?,
            env: Vec::new(), // child inherits SEPIA_*/driver envs from the node
        };
        let service = peregrin_session::SepiaService::spawn(spec)
            .await
            .map_err(|e| std::io::Error::other(format!("sepia serve failed: {e}")))?;
        let plane: Arc<dyn SessionPlane> = Arc::new(peregrin_session::ServicePlane(service));
        let state = AppState::from_env(env, Arc::clone(&plane));
        return run(env, state, plane).await;
    }

    let paths = Paths::new(env.home.clone());
    let mut options = base_options();
    options.idle_ttl = Some(env.idle_ttl);
    options.sweep_interval = Some(env.sweep);
    options.file_history_dir = Some(env.claude_dir.join("file-history"));
    let local = build(&paths, options).await?;
    let plane: Arc<dyn SessionPlane> =
        Arc::new(peregrin_session::LocalPlane(Arc::clone(&local.plane)));

    // Conversion seams — devin↔cline.
    let devin_store = local
        .registry
        .merged_store()
        .await
        .for_agent("devin")
        .map(|s| Arc::new(s.clone()) as Arc<dyn sepia_core::storage::SessionRepository>);
    let cline_dir = env.cline_dir.clone();
    let state = if let Some(devin) = &devin_store {
        let repo = Arc::clone(devin);
        let cline = cline_dir.clone();
        let convert = Arc::new(move |id: String, target: ImportTarget| {
            let repo = Arc::clone(&repo);
            let cline = cline.clone();
            Box::pin(async move {
                match target {
                    ImportTarget::Cline => {
                        sepia_convert::install_cline(&repo, &id, &cline, None, false)
                            .await
                            .map_err(|e| control_err(e.message))
                    }
                    ImportTarget::Devin => sepia_convert::import_cline(
                        &cline.join("sessions").join(&id),
                        None,
                        &repo,
                        false,
                    )
                    .await
                    .map_err(|e| control_err(e.message)),
                }
            }) as futures::future::BoxFuture<'static, Result<String, ControlError>>
        }) as peregrin_http::ConvertSession;
        let repo2 = Arc::clone(devin);
        let import = Arc::new(move |session: Session, _target: ImportTarget| {
            let repo = Arc::clone(&repo2);
            Box::pin(async move {
                sepia_convert::import_session(&repo, &session)
                    .await
                    .map_err(|e| control_err(e.message))
            }) as futures::future::BoxFuture<'static, Result<String, ControlError>>
        }) as peregrin_http::ImportSession;
        AppState::from_env(env, Arc::clone(&plane))
            .with_convert(convert)
            .with_import_session(import)
    } else {
        AppState::from_env(env, Arc::clone(&plane))
    };
    run(env, state, plane).await
}

async fn run(env: &Env, state: AppState, plane: Arc<dyn SessionPlane>) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind((env.host.as_str(), env.port)).await?;
    tracing::info!("peregrin-node listening on {}:{}", env.host, env.port);
    let app = peregrin_http::app(state);
    // `ConnectInfo` feeds the /api/pair limiter its client-IP key.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("peregrin-node shutting down");
        plane.close_all().await;
    })
    .await
}
