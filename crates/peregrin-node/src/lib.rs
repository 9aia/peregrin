//! peregrin-node — the machine daemon. Spawns `sepia serve` as the
//! session plane (`PEREGRIN_SESSION_PLANE=local`/`inprocess` opts back
//! into linking `sepia-session` in-process, the dev mode), mounts the
//! `/api/*` surface, and shuts the plane down on SIGINT/SIGTERM.
//! Orchestration/policy layers sit above this: campaigns, multi-node
//! arbitration, the hub.

use std::path::PathBuf;
use std::sync::Arc;

use futures::future::BoxFuture;
use peregrin_http::AppState;
use peregrin_http::env::Env;
use peregrin_http::routes::sessions::ImportTarget;
use peregrin_session::SessionPlane;
use sepia_core::Session;
use sepia_session::{ImportOverrides, Paths, base_options, build};

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

/// Build + serve the node — the whole `peregrin-node` boot path, shared
/// with `peregrin serve` so both binaries behave identically. Call on
/// a tokio runtime; returns when the listener fails or after graceful
/// shutdown.
///
/// # Errors
/// `std::io::Error` on plane build, bind, or serve failure.
pub async fn serve(env: &Env) -> std::io::Result<()> {
    // `PEREGRIN_SESSION_PLANE` selects the wire mode: unset or anything
    // but `local`/`inprocess` spawns `sepia serve` — the session plane
    // is just another managed binary. Coverage builds compile the
    // spawn arm out (tests can't set process env under
    // `forbid(unsafe_code)`); the spawned-node e2e runs it instead.
    #[cfg(not(coverage))]
    if !matches!(
        std::env::var("PEREGRIN_SESSION_PLANE").as_deref(),
        Ok("local" | "inprocess")
    ) {
        let spec = peregrin_session::Spec {
            binary: peregrin_session::sepia_binary().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "sepia binary not found — set PEREGRIN_SEPIA or install sepia",
                )
            })?,
            // The child inherits the SEPIA_*/driver envs; forwarding the
            // node's resolved cline dir keeps `PEREGRIN_CLINE_DIR`
            // meaningful for the cline driver's read/write store.
            env: vec![(
                "SEPIA_CLINE_DIR".into(),
                env.cline_dir.display().to_string(),
            )],
        };
        let service = peregrin_session::SepiaService::spawn(spec)
            .await
            .map_err(|e| std::io::Error::other(format!("sepia serve failed: {e}")))?;
        let plane: Arc<dyn SessionPlane> = Arc::new(peregrin_session::ServicePlane(service));
        let state = app_state(env, &plane);
        return run(env, state, plane).await;
    }

    let paths = Paths::new(env.home.clone());
    let mut options = base_options();
    options.idle_ttl = Some(env.idle_ttl);
    options.sweep_interval = Some(env.sweep);
    options.file_history_dir = Some(env.claude_dir.join("file-history"));
    let local = build(&paths, options).await?;
    let plane: Arc<dyn SessionPlane> = Arc::new(peregrin_session::LocalPlane(
        Arc::clone(&local.plane),
        Some(local),
    ));
    let state = app_state(env, &plane);
    run(env, state, plane).await
}

/// The `AppState` both plane modes mount — convert/import ride the
/// `SessionPlane` itself (`sessions.convert`/`sessions.import` over
/// `sepia serve`, `Local`'s driver-wire ops in-process), so the seam
/// wiring is identical either way.
fn app_state(env: &Env, plane: &Arc<dyn SessionPlane>) -> AppState {
    let convert = {
        let plane = Arc::clone(plane);
        Arc::new(move |id: String, target: ImportTarget| {
            let plane = Arc::clone(&plane);
            Box::pin(async move {
                // A cline copy reads the devin store, a devin copy
                // reads cline's — `from` keeps the source explicit.
                let from = match target {
                    ImportTarget::Cline => "devin",
                    ImportTarget::Devin => "cline",
                };
                plane
                    .convert_session(&id, target.as_str(), Some(from))
                    .await
            }) as BoxFuture<'static, Result<String, sepia_session::ControlError>>
        }) as peregrin_http::ConvertSession
    };
    let import = {
        let plane = Arc::clone(plane);
        Arc::new(move |session: Session, target: ImportTarget| {
            let plane = Arc::clone(&plane);
            Box::pin(async move {
                plane
                    .import_session(target.as_str(), &session, &ImportOverrides::default())
                    .await
            }) as BoxFuture<'static, Result<String, sepia_session::ControlError>>
        }) as peregrin_http::ImportSession
    };
    AppState::from_env(env, Arc::clone(plane))
        .with_convert(convert)
        .with_import_session(import)
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
