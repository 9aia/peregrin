use handl_vm_values_json::NonFunctionValue;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use handl_vm::{Context, Function};
use handl_vm_common::Module;
use handl_vm_values_json::{Error, FunctionValue};

use crate::error::HttpServerError;
use crate::http_method::HttpMethod;
use crate::request::HttpServerRequest;
use crate::response::HttpServerResponse;

// ── HttpServer trait ────────────────────────────────────────────────

pub type Handler = Box<dyn Fn(HttpServerRequest) -> HttpServerResponse + Send + Sync>;

/// Abstract HTTP server service.
pub trait HttpServer: Send + Sync {
    /// Start accepting connections on `addr`.
    fn start(&self, addr: &str) -> Result<(), HttpServerError>;

    /// Signal the server to stop accepting new connections.
    fn stop(&self) -> Result<(), HttpServerError>;

    /// Block until the server has fully shut down.
    fn join(&self) -> Result<(), HttpServerError>;

    /// Returns `true` if the server is currently listening.
    fn is_running(&self) -> bool;

    /// Register a handler for a method/path pattern.
    fn route(
        &self,
        method: HttpMethod,
        path: &str,
        handler: Handler,
    ) -> Result<(), HttpServerError>;

    /// Register an handl `FunctionValue` as a handler for a method/path pattern.
    fn route_function(
        &self,
        method: HttpMethod,
        path: &str,
        handler: FunctionValue,
    ) -> Result<(), HttpServerError>;

    /// Serve static files from `dir` under `path`.
    fn serve_static(&self, path: &str, dir: &str) -> Result<(), HttpServerError>;
}

// ── Stub implementation for tests ───────────────────────────────────

#[derive(Default)]
pub struct StubHttpServer {
    state: Mutex<StubState>,
}

#[derive(Default)]
struct StubState {
    running: bool,
    routes: HashMap<(HttpMethod, String), Handler>,
    static_routes: HashMap<String, String>,
}

impl StubHttpServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn response_for(&self, req: HttpServerRequest) -> Option<HttpServerResponse> {
        let routes = self.state.lock().unwrap();
        routes
            .routes
            .get(&(req.method, req.path.clone()))
            .map(|h| h(req))
    }
}

impl HttpServer for StubHttpServer {
    fn start(&self, _addr: &str) -> Result<(), HttpServerError> {
        let mut state = self.state.lock().unwrap();
        if state.running {
            return Err(HttpServerError::AlreadyRunning);
        }
        state.running = true;
        Ok(())
    }

    fn stop(&self) -> Result<(), HttpServerError> {
        let mut state = self.state.lock().unwrap();
        if !state.running {
            return Err(HttpServerError::NotRunning);
        }
        state.running = false;
        Ok(())
    }

    fn join(&self) -> Result<(), HttpServerError> {
        let mut state = self.state.lock().unwrap();
        if !state.running {
            return Err(HttpServerError::NotRunning);
        }
        state.running = false;
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.state.lock().unwrap().running
    }

    fn route(
        &self,
        method: HttpMethod,
        path: &str,
        handler: Handler,
    ) -> Result<(), HttpServerError> {
        self.state
            .lock()
            .unwrap()
            .routes
            .insert((method, path.to_string()), handler);
        Ok(())
    }

    fn route_function(
        &self,
        method: HttpMethod,
        path: &str,
        _handler: FunctionValue,
    ) -> Result<(), HttpServerError> {
        let _ = (method, path);
        Ok(())
    }

    fn serve_static(&self, path: &str, dir: &str) -> Result<(), HttpServerError> {
        self.state
            .lock()
            .unwrap()
            .static_routes
            .insert(path.to_string(), dir.to_string());
        Ok(())
    }
}

// ── Module adapter ──────────────────────────────────────────────────

/// Module adapter for [`HttpServer`] — exposes `http_server.*` functions.
///
/// | Function | Args | Description |
/// |----------|------|-------------|
/// | `http_server.new()` | `()` | Creates a new server and returns its id. |
/// | `http_server.start(server, addr)` | `(server, addr)` | Starts the server. |
/// | `http_server.stop(server)` | `(server)` | Stops the server. |
/// | `http_server.join(server)` | `(server)` | Blocks until shutdown. |
/// | `http_server.route(server, method, path, body)` | `(server, method, path, body)` | Registers a static response route. |
/// | `http_server.serve_static(server, path, dir)` | `(server, path, dir)` | Serves static files. |
pub struct HttpServerModule<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> {
    new_fn: NewFn<F>,
    start_fn: StartFn<F>,
    stop_fn: StopFn<F>,
    join_fn: JoinFn<F>,
    route_fn: RouteFn<F>,
    serve_static_fn: ServeStaticFn<F>,
}

impl<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> HttpServerModule<F> {
    pub fn new(factory: F) -> Self {
        let inner = Arc::new(Inner::new(factory));
        HttpServerModule {
            new_fn: NewFn {
                inner: inner.clone(),
            },
            start_fn: StartFn {
                inner: inner.clone(),
            },
            stop_fn: StopFn {
                inner: inner.clone(),
            },
            join_fn: JoinFn {
                inner: inner.clone(),
            },
            route_fn: RouteFn {
                inner: inner.clone(),
            },
            serve_static_fn: ServeStaticFn { inner },
        }
    }
}

impl<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> Module for HttpServerModule<F> {
    fn name(&self) -> &str {
        "http_server"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "new" => Some(&self.new_fn),
            "start" => Some(&self.start_fn),
            "stop" => Some(&self.stop_fn),
            "join" => Some(&self.join_fn),
            "route" => Some(&self.route_fn),
            "serve_static" => Some(&self.serve_static_fn),
            _ => None,
        }
    }
}

struct Inner<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> {
    factory: F,
    servers: Mutex<HashMap<usize, Arc<dyn HttpServer>>>,
    next_id: AtomicUsize,
    _phantom: PhantomData<F>,
}

impl<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> Inner<F> {
    fn new(factory: F) -> Self {
        Inner {
            factory,
            servers: Mutex::new(HashMap::new()),
            next_id: AtomicUsize::new(1),
            _phantom: PhantomData,
        }
    }

    fn create_server(&self) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let server = (self.factory)();
        self.servers.lock().unwrap().insert(id, server);
        id
    }

    fn with_server<T>(
        &self,
        id: usize,
        f: impl FnOnce(&Arc<dyn HttpServer>) -> Result<T, HttpServerError>,
    ) -> Result<T, HttpServerError> {
        let servers = self.servers.lock().unwrap();
        let server = servers
            .get(&id)
            .ok_or_else(|| HttpServerError::InvalidArgument {
                message: format!("unknown server id: {id}"),
            })?;
        f(server)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn require_string_at(args: &[NonFunctionValue], idx: usize) -> Result<&str, Error> {
    args.get(idx)
        .ok_or(Error::ArgumentCount {
            expected: idx + 1,
            got: args.len(),
        })?
        .as_str()
        .ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })
}

fn require_id_at(args: &[NonFunctionValue], idx: usize) -> Result<usize, Error> {
    let value = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    let n = value.as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number",
    })?;
    Ok(n as usize)
}

// ── Verb function structs ───────────────────────────────────────────

struct NewFn<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> {
    inner: Arc<Inner<F>>,
}

impl<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> Function<NonFunctionValue>
    for NewFn<F>
{
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let id = self.inner.create_server();
        Ok(NonFunctionValue::new(id as f64))
    }
}

struct StartFn<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> {
    inner: Arc<Inner<F>>,
}

impl<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> Function<NonFunctionValue>
    for StartFn<F>
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let id = require_id_at(args, 0)?;
        let addr = require_string_at(args, 1)?;
        self.inner
            .with_server(id, |server| server.start(addr))
            .map_err(Into::into)
            .map(|_| NonFunctionValue::new(serde_json::Value::Null))
    }
}

struct StopFn<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> {
    inner: Arc<Inner<F>>,
}

impl<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> Function<NonFunctionValue>
    for StopFn<F>
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let id = require_id_at(args, 0)?;
        self.inner
            .with_server(id, |server| server.stop())
            .map_err(Into::into)
            .map(|_| NonFunctionValue::new(serde_json::Value::Null))
    }
}

struct JoinFn<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> {
    inner: Arc<Inner<F>>,
}

impl<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> Function<NonFunctionValue>
    for JoinFn<F>
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let id = require_id_at(args, 0)?;
        self.inner
            .with_server(id, |server| server.join())
            .map_err(Into::into)
            .map(|_| NonFunctionValue::new(serde_json::Value::Null))
    }
}

struct RouteFn<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> {
    inner: Arc<Inner<F>>,
}

impl<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> Function<NonFunctionValue>
    for RouteFn<F>
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let id = require_id_at(args, 0)?;
        let method_str = require_string_at(args, 1)?;
        let method = HttpMethod::parse(method_str)
            .ok_or_else(|| Error::PrimitiveError(format!("unknown HTTP method: {method_str}")))?;
        let path = require_string_at(args, 2)?;
        let handler_arg = args.get(3).ok_or(Error::ArgumentCount {
            expected: 4,
            got: args.len(),
        })?;

        if let Some(function_value) = FunctionValue::from_json(handler_arg.inner()) {
            self.inner
                .with_server(id, |server| {
                    server.route_function(method, path, function_value)
                })
                .map_err(Error::from)?;
        } else {
            let body = handler_arg.as_str().ok_or(Error::TypeMismatch {
                expected: "string or function",
                got: "non-string non-function",
            })?;
            let body = body.to_string();
            let handler: Handler =
                Box::new(move |_req: HttpServerRequest| HttpServerResponse::new(200, body.clone()));
            self.inner
                .with_server(id, |server| server.route(method, path, handler))
                .map_err(Error::from)?;
        }

        Ok(NonFunctionValue::new(serde_json::Value::Null))
    }
}

struct ServeStaticFn<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> {
    inner: Arc<Inner<F>>,
}

impl<F: Fn() -> Arc<dyn HttpServer> + Send + Sync + 'static> Function<NonFunctionValue>
    for ServeStaticFn<F>
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let id = require_id_at(args, 0)?;
        let path = require_string_at(args, 1)?;
        let dir = require_string_at(args, 2)?;
        self.inner
            .with_server(id, |server| server.serve_static(path, dir))
            .map_err(Into::into)
            .map(|_| NonFunctionValue::new(serde_json::Value::Null))
    }
}

#[cfg(test)]
mod tests;
