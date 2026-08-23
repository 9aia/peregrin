use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tokio::sync::oneshot;

use ac_lib_http_server::{
    Handler, HttpMethod, HttpServer, HttpServerError, HttpServerRequest, HttpServerResponse,
};

/// Production-baseline HTTP server backed by `tokio` + `hyper`.
pub struct StdHttpServer {
    state: Arc<ServerState>,
}

#[derive(Default)]
struct ServerState {
    routes: Mutex<HashMap<(HttpMethod, String), Handler>>,
    static_routes: Mutex<HashMap<String, String>>,
    runtime: Mutex<Option<ServerRuntime>>,
    local_addr: Mutex<Option<SocketAddr>>,
}

struct ServerRuntime {
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: JoinHandle<()>,
}

impl StdHttpServer {
    pub fn new() -> Self {
        StdHttpServer {
            state: Arc::new(ServerState::default()),
        }
    }

    pub fn new_arc() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl Default for StdHttpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl StdHttpServer {
    /// Returns the local address the server is bound to, if running.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        *self.state.local_addr.lock().unwrap()
    }
}

impl HttpServer for StdHttpServer {
    fn start(&self, addr: &str) -> Result<(), HttpServerError> {
        let mut runtime = self.state.runtime.lock().unwrap();
        if runtime.is_some() {
            return Err(HttpServerError::AlreadyRunning);
        }

        let listener = TcpListener::bind(addr).map_err(|e| HttpServerError::Network {
            message: e.to_string(),
        })?;
        listener
            .set_nonblocking(true)
            .map_err(|e| HttpServerError::Network {
                message: e.to_string(),
            })?;
        *self.state.local_addr.lock().unwrap() = listener.local_addr().ok();

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let state = self.state.clone();
        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");
            rt.block_on(run_server(listener, state, shutdown_rx));
        });

        *runtime = Some(ServerRuntime {
            shutdown_tx: Some(shutdown_tx),
            handle,
        });

        Ok(())
    }

    fn stop(&self) -> Result<(), HttpServerError> {
        let mut runtime = self.state.runtime.lock().unwrap();
        match runtime.as_mut() {
            Some(ServerRuntime { shutdown_tx, .. }) => {
                if let Some(tx) = shutdown_tx.take() {
                    let _ = tx.send(());
                    Ok(())
                } else {
                    Err(HttpServerError::NotRunning)
                }
            }
            None => Err(HttpServerError::NotRunning),
        }
    }

    fn join(&self) -> Result<(), HttpServerError> {
        let mut runtime = self.state.runtime.lock().unwrap();
        match runtime.take() {
            Some(ServerRuntime { handle, .. }) => {
                handle.join().map_err(|_| HttpServerError::Handler {
                    message: "server thread panicked".to_string(),
                })?;
                *self.state.local_addr.lock().unwrap() = None;
                Ok(())
            }
            None => Err(HttpServerError::NotRunning),
        }
    }

    fn is_running(&self) -> bool {
        self.state
            .runtime
            .lock()
            .unwrap()
            .as_ref()
            .map_or(false, |r| !r.handle.is_finished())
    }

    fn route(
        &self,
        method: HttpMethod,
        path: &str,
        handler: Handler,
    ) -> Result<(), HttpServerError> {
        self.state
            .routes
            .lock()
            .unwrap()
            .insert((method, path.to_string()), handler);
        Ok(())
    }

    fn serve_static(&self, path: &str, dir: &str) -> Result<(), HttpServerError> {
        self.state
            .static_routes
            .lock()
            .unwrap()
            .insert(path.to_string(), dir.to_string());
        Ok(())
    }
}

async fn run_server(
    listener: TcpListener,
    state: Arc<ServerState>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    let listener = tokio::net::TcpListener::from_std(listener).expect("convert to tokio listener");

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let state = state.clone();
                        tokio::spawn(handle_connection(stream, state));
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

async fn handle_connection(stream: TcpStream, state: Arc<ServerState>) {
    let io = TokioIo::new(stream);
    let service = service_fn(move |req: Request<Incoming>| {
        let state = state.clone();
        async move { Ok::<_, Infallible>(handle_request(req, state).await) }
    });
    let conn = http1::Builder::new().serve_connection(io, service);
    if let Err(e) = conn.await {
        eprintln!("connection error: {e}");
    }
}

async fn handle_request(req: Request<Incoming>, state: Arc<ServerState>) -> Response<Full<Bytes>> {
    let method = HttpMethod::parse(req.method().as_str()).unwrap_or(HttpMethod::Get);
    let path = req.uri().path().to_string();
    let query = parse_query(req.uri().query().unwrap_or(""));
    let headers = extract_headers(req.headers());
    let body = match body_string(req.into_body()).await {
        Ok(b) => b,
        Err(_) => return error_response(400, "bad request body"),
    };

    let server_request = HttpServerRequest {
        method,
        path: path.clone(),
        query,
        headers,
        body,
    };

    {
        let routes = state.routes.lock().unwrap();
        if let Some(handler) = routes.get(&(method, path.clone())) {
            let response = handler(server_request);
            return convert_response(response);
        }
    }

    if method == HttpMethod::Get {
        let static_match = {
            let static_routes = state.static_routes.lock().unwrap();
            static_routes.iter().find_map(|(prefix, dir)| {
                path.strip_prefix(prefix)
                    .map(|rest| (prefix.clone(), dir.clone(), rest.to_string()))
            })
        };
        if let Some((_, dir, rest)) = static_match {
            let rest = rest.trim_start_matches('/');
            if rest.split('/').any(|c| c == "..") {
                return error_response(403, "forbidden");
            }
            let resolved = Path::new(&dir).join(rest);
            match tokio::fs::read(&resolved).await {
                Ok(content) => {
                    let mut response = Response::builder().status(200);
                    if let Some(guess) = mime_guess(&resolved) {
                        response = response.header(hyper::header::CONTENT_TYPE, guess);
                    }
                    return response
                        .body(Full::new(Bytes::from(content)))
                        .unwrap_or_else(|_| error_response(500, "internal error"));
                }
                Err(_) => return error_response(404, "not found"),
            }
        }
    }

    error_response(404, "not found")
}

async fn body_string(body: Incoming) -> Result<String, HttpServerError> {
    let collected = body.collect().await.map_err(|e| HttpServerError::Handler {
        message: e.to_string(),
    })?;
    let bytes = collected.to_bytes();
    String::from_utf8(bytes.to_vec()).map_err(|e| HttpServerError::Handler {
        message: e.to_string(),
    })
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

fn extract_headers(headers: &hyper::HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect()
}

fn convert_response(response: HttpServerResponse) -> Response<Full<Bytes>> {
    let mut builder = Response::builder().status(response.status);
    for (k, v) in &response.headers {
        builder = builder.header(k, v);
    }
    builder
        .body(Full::new(Bytes::from(response.body)))
        .unwrap_or_else(|_| error_response(500, "internal error"))
}

fn error_response(status: u16, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(hyper::header::CONTENT_TYPE, "text/plain")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("internal error"))))
}

fn mime_guess(path: &PathBuf) -> Option<&'static str> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "html" | "htm" => Some("text/html"),
        "css" => Some("text/css"),
        "js" => Some("text/javascript"),
        "json" => Some("application/json"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "svg" => Some("image/svg+xml"),
        _ => Some("text/plain"),
    }
}

fn factory() -> Arc<dyn HttpServer> {
    Arc::new(StdHttpServer::new())
}

/// Preconfigured `HttpServerModule` that builds [`StdHttpServer`] instances.
pub type StdHttpServerModule = ac_lib_http_server::HttpServerModule<fn() -> Arc<dyn HttpServer>>;

/// Creates a new [`StdHttpServerModule`].
pub fn std_http_server_module() -> StdHttpServerModule {
    ac_lib_http_server::HttpServerModule::new(factory)
}
