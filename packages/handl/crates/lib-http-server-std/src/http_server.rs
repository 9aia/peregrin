use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto;
use tokio::net::TcpStream;
use tokio::sync::oneshot;

use handl_lib_http_server::{
    Handler, HttpMethod, HttpServer, HttpServerError, HttpServerRequest, HttpServerResponse,
};
use handl_vm_backend_rust_interpreter::CanonicalEval;
use handl_vm_host_canonical::CanonicalHost;
use handl_vm_values_json::{Error as VmError, Eval, FunctionValue, NonFunctionValue};
use serde_json::Value as JsonValue;

/// Runtime configuration for [`StdHttpServer`].
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Maximum request body size in bytes (default 1 MiB).
    pub body_limit: usize,
    /// Time to wait for a new request on an idle connection (default 30 s).
    pub idle_timeout: Duration,
    /// Whether to enable HTTP/1.1 keep-alive (default `true`).
    pub keep_alive: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            body_limit: 1_048_576,
            idle_timeout: Duration::from_secs(30),
            keep_alive: true,
        }
    }
}

/// Production-baseline HTTP server backed by `tokio` + `hyper`.
pub struct StdHttpServer {
    state: Arc<ServerState>,
}

struct ServerState {
    routes: Mutex<HashMap<HttpMethod, matchit::Router<Handler>>>,
    static_routes: Mutex<HashMap<String, String>>,
    runtime: Mutex<Option<ServerRuntime>>,
    local_addr: Mutex<Option<SocketAddr>>,
    config: ServerConfig,
    eval: Arc<CanonicalEval<CanonicalHost>>,
}

impl Default for ServerState {
    fn default() -> Self {
        ServerState {
            routes: Mutex::new(HashMap::new()),
            static_routes: Mutex::new(HashMap::new()),
            runtime: Mutex::new(None),
            local_addr: Mutex::new(None),
            config: ServerConfig::default(),
            eval: Arc::new(CanonicalEval::new(CanonicalHost::default(), None, None)),
        }
    }
}

struct ServerRuntime {
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: JoinHandle<()>,
}

impl StdHttpServer {
    pub fn new() -> Self {
        Self::with_config(ServerConfig::default())
    }

    pub fn with_config(config: ServerConfig) -> Self {
        StdHttpServer {
            state: Arc::new(ServerState {
                config,
                ..ServerState::default()
            }),
        }
    }

    pub fn new_arc() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn new_arc_with_config(config: ServerConfig) -> Arc<Self> {
        Arc::new(Self::with_config(config))
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
        let mut routes = self.state.routes.lock().unwrap();
        let router = routes.entry(method).or_insert_with(matchit::Router::new);
        router
            .insert(path, handler)
            .map_err(|e| HttpServerError::InvalidArgument {
                message: e.to_string(),
            })
    }

    fn route_function(
        &self,
        method: HttpMethod,
        path: &str,
        function_value: FunctionValue,
    ) -> Result<(), HttpServerError> {
        let eval = self.state.eval.clone();
        let handler: Handler = Box::new(move |req: HttpServerRequest| {
            let mut vars: Vec<(String, NonFunctionValue)> = function_value
                .params
                .iter()
                .map(|(name, _)| {
                    let value = req
                        .params
                        .get(name)
                        .map(|s| NonFunctionValue::new(s.clone()))
                        .unwrap_or_default();
                    (name.clone(), value)
                })
                .collect();
            vars.push(("request".to_string(), request_to_value(&req)));

            match eval.eval_expr(&function_value.body, &vars) {
                Ok(value) => value_to_response(&value),
                Err(VmError::PrimitiveError(msg)) => HttpServerResponse::new(500, msg),
                Err(e) => HttpServerResponse::new(500, e.to_string()),
            }
        });
        self.route(method, path, handler)
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
    let mut join_set = tokio::task::JoinSet::new();

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let state = state.clone();
                        join_set.spawn(handle_connection(stream, state));
                    }
                    Err(_) => break,
                }
            }
        }
    }

    while let Some(res) = join_set.join_next().await {
        if let Err(e) = res {
            eprintln!("connection task failed: {e}");
        }
    }
}

async fn handle_connection(stream: TcpStream, state: Arc<ServerState>) {
    let io = TokioIo::new(stream);
    let config = state.config.clone();
    let service = service_fn(move |req: Request<Incoming>| {
        let state = state.clone();
        async move { Ok::<_, Infallible>(handle_request(req, state).await) }
    });

    let mut builder = auto::Builder::new(TokioExecutor::new());
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(Some(config.idle_timeout))
        .keep_alive(config.keep_alive);
    builder
        .http2()
        .timer(TokioTimer::new())
        .max_concurrent_streams(256)
        .keep_alive_interval(if config.keep_alive {
            Some(config.idle_timeout / 4)
        } else {
            None
        })
        .keep_alive_timeout(config.idle_timeout);

    let conn = builder.serve_connection(io, service);
    if let Err(e) = conn.await {
        eprintln!("connection error: {e}");
    }
}

async fn handle_request(req: Request<Incoming>, state: Arc<ServerState>) -> Response<Full<Bytes>> {
    let method = HttpMethod::parse(req.method().as_str()).unwrap_or(HttpMethod::Get);
    let path = req.uri().path().to_string();
    let query = parse_query(req.uri().query().unwrap_or(""));
    let headers = extract_headers(req.headers());
    let body_limit = state.config.body_limit;
    let body = match body_string(req.into_body(), body_limit).await {
        Ok(b) => b,
        Err(HttpServerError::PayloadTooLarge { .. }) => {
            return error_response(413, "payload too large");
        }
        Err(_) => return error_response(400, "bad request body"),
    };

    {
        let routes = state.routes.lock().unwrap();
        if let Some(router) = routes.get(&method) {
            match router.at(&path) {
                Ok(matched) => {
                    let params = matched
                        .params
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect();
                    let server_request = HttpServerRequest {
                        method,
                        path: path.clone(),
                        params,
                        query,
                        headers,
                        body,
                    };
                    let response = (matched.value)(server_request);
                    return convert_response(response);
                }
                Err(_) => {}
            }
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
                    let content_type = mime_guess::from_path(&resolved).first_or_octet_stream();
                    return Response::builder()
                        .status(200)
                        .header(hyper::header::CONTENT_TYPE, content_type.as_ref())
                        .body(Full::new(Bytes::from(content)))
                        .unwrap_or_else(|_| error_response(500, "internal error"));
                }
                Err(_) => return error_response(404, "not found"),
            }
        }
    }

    error_response(404, "not found")
}

async fn body_string(body: Incoming, limit: usize) -> Result<String, HttpServerError> {
    let collected = body.collect().await.map_err(|e| HttpServerError::Handler {
        message: e.to_string(),
    })?;
    let bytes = collected.to_bytes();
    if bytes.len() > limit {
        return Err(HttpServerError::PayloadTooLarge { limit });
    }
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
    if !response
        .headers
        .keys()
        .any(|k| k.eq_ignore_ascii_case("content-type"))
    {
        builder = builder.header(hyper::header::CONTENT_TYPE, "text/plain; charset=utf-8");
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

fn request_to_value(req: &HttpServerRequest) -> NonFunctionValue {
    NonFunctionValue::new(JsonValue::Object({
        let mut map = serde_json::Map::new();
        map.insert(
            "method".to_string(),
            JsonValue::String(req.method.as_str().to_string()),
        );
        map.insert("path".to_string(), JsonValue::String(req.path.clone()));
        map.insert(
            "params".to_string(),
            JsonValue::Object(
                req.params
                    .iter()
                    .map(|(k, v)| (k.clone(), JsonValue::String(v.clone())))
                    .collect(),
            ),
        );
        map.insert(
            "query".to_string(),
            JsonValue::Object(
                req.query
                    .iter()
                    .map(|(k, v)| (k.clone(), JsonValue::String(v.clone())))
                    .collect(),
            ),
        );
        map.insert(
            "headers".to_string(),
            JsonValue::Object(
                req.headers
                    .iter()
                    .map(|(k, v)| (k.clone(), JsonValue::String(v.clone())))
                    .collect(),
            ),
        );
        map.insert("body".to_string(), JsonValue::String(req.body.clone()));
        map
    }))
}

fn value_to_response(value: &NonFunctionValue) -> HttpServerResponse {
    let body = value
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| value.as_number().map(|n| n.to_string()))
        .or_else(|| {
            let inner = value.inner();
            if inner.is_null() {
                Some(String::new())
            } else {
                Some(serde_json::to_string(inner).unwrap_or_default())
            }
        })
        .unwrap_or_default();
    HttpServerResponse::new(200, body)
}

fn factory() -> Arc<dyn HttpServer> {
    Arc::new(StdHttpServer::new())
}

/// Preconfigured `HttpServerModule` that builds [`StdHttpServer`] instances.
pub type StdHttpServerModule = handl_lib_http_server::HttpServerModule<fn() -> Arc<dyn HttpServer>>;

/// Creates a new [`StdHttpServerModule`].
pub fn std_http_server_module() -> StdHttpServerModule {
    handl_lib_http_server::HttpServerModule::new(factory)
}
