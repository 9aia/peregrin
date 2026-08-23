use std::sync::Arc;

use ac_runtime::NonFunctionValue;
use ac_runtime_modules::Module;
use ac_runtime::{Context, Error, Function};

use crate::error::OsError;

/// Abstract socket service.
///
/// Implementations:
/// - `ac-lib-os-std` — backed by `std::net`
/// - [`socket_stub::StubSocket`] — in-memory for testing
pub trait Socket: Send + Sync {
    fn connect(&self, address: &str) -> Result<i64, OsError>;
    fn send(&self, handle: i64, data: &str) -> Result<usize, OsError>;
    fn recv(&self, handle: i64, length: usize) -> Result<String, OsError>;
    fn close(&self, handle: i64) -> Result<(), OsError>;
}

/// Module adapter for [`Socket`] — exposes `socket.*` functions.
pub struct SocketModule {
    connect: ConnectFn,
    send: SendFn,
    recv: RecvFn,
    close: CloseFn}

impl SocketModule {
    pub fn new(socket: impl Socket + 'static) -> Self {
        let socket = Arc::new(socket);
        SocketModule {
            connect: ConnectFn { socket: socket.clone() },
            send: SendFn { socket: socket.clone() },
            recv: RecvFn { socket: socket.clone() },
            close: CloseFn { socket }}
    }
}

impl Module for SocketModule {
    fn name(&self) -> &str {
        "socket"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "connect" => Some(&self.connect),
            "send" => Some(&self.send),
            "recv" => Some(&self.recv),
            "close" => Some(&self.close),
            _ => None}
    }
}

fn require_string_arg(args: &[NonFunctionValue]) -> Result<&str, Error> {
    args.first()
        .ok_or(Error::ArgumentCount { expected: 1, got: 0 })?
        .as_str()
        .ok_or(Error::TypeMismatch { expected: "string", got: "non-string" })
}

fn require_number_arg(args: &[NonFunctionValue]) -> Result<i64, Error> {
    let n = args
        .first()
        .ok_or(Error::ArgumentCount { expected: 1, got: 0 })?
        .as_number()
        .ok_or(Error::TypeMismatch { expected: "number", got: "non-number" })?;
    Ok(n as i64)
}

struct ConnectFn {
    socket: Arc<dyn Socket>}

impl Function for ConnectFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let address = require_string_arg(args)?;
        self.socket
            .connect(address)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct SendFn {
    socket: Arc<dyn Socket>}

impl Function for SendFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount { expected: 2, got: args.len() });
        }
        let handle = require_number_arg(args)?;
        let data = args[1].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string"})?;
        self.socket
            .send(handle, data)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct RecvFn {
    socket: Arc<dyn Socket>}

impl Function for RecvFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount { expected: 2, got: args.len() });
        }
        let handle = require_number_arg(args)?;
        let length = args[1]
            .as_number()
            .ok_or(Error::TypeMismatch { expected: "number", got: "non-number" })? as usize;
        self.socket
            .recv(handle, length)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct CloseFn {
    socket: Arc<dyn Socket>}

impl Function for CloseFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let handle = require_number_arg(args)?;
        self.socket.close(handle)?;
        Ok(NonFunctionValue::default())
    }
}
