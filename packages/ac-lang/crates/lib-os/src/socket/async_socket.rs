use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use ac_runtime_values::{Error, NonFunctionValue};

use ac_runtime::{AsyncFunction, Context, Function};
use ac_runtime_modules::Module;

use crate::error::OsError;

/// A boxed, sendable future for async socket operations.
pub type AsyncSocketFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, OsError>> + Send + 'a>>;

/// Abstract async socket service.
///
/// Implementations:
/// - `ac-lib-os-std` — `MioSocket` backed by `mio` (Unix only).
pub trait AsyncSocket: Send + Sync {
    /// Asynchronously connect to `address` and return a handle.
    fn connect<'a>(&'a self, address: &'a str) -> AsyncSocketFuture<'a, i64>;

    /// Asynchronously send `data` on `handle` and return the bytes written.
    fn send<'a>(&'a self, handle: i64, data: &'a str) -> AsyncSocketFuture<'a, usize>;

    /// Asynchronously receive up to `length` bytes from `handle`.
    fn recv<'a>(&'a self, handle: i64, length: usize) -> AsyncSocketFuture<'a, String>;

    /// Asynchronously close `handle`.
    fn close<'a>(&'a self, handle: i64) -> AsyncSocketFuture<'a, ()>;
}

/// Module adapter for [`AsyncSocket`] — exposes `socket.*` async functions.
pub struct AsyncSocketModule {
    connect: AsyncConnectFn,
    send: AsyncSendFn,
    recv: AsyncRecvFn,
    close: AsyncCloseFn,
}

impl AsyncSocketModule {
    pub fn new(socket: impl AsyncSocket + 'static) -> Self {
        let socket = Arc::new(socket);
        AsyncSocketModule {
            connect: AsyncConnectFn {
                socket: socket.clone(),
            },
            send: AsyncSendFn {
                socket: socket.clone(),
            },
            recv: AsyncRecvFn {
                socket: socket.clone(),
            },
            close: AsyncCloseFn { socket },
        }
    }
}

impl Module for AsyncSocketModule {
    fn name(&self) -> &str {
        "socket"
    }

    fn function(&self, _name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        None
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction<NonFunctionValue>> {
        match name {
            "connect" => Some(&self.connect),
            "send" => Some(&self.send),
            "recv" => Some(&self.recv),
            "close" => Some(&self.close),
            _ => None,
        }
    }
}

fn require_string_arg(args: &[NonFunctionValue]) -> Result<&str, Error> {
    args.first()
        .ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?
        .as_str()
        .ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })
}

fn require_number_arg(args: &[NonFunctionValue]) -> Result<i64, Error> {
    args.first()
        .ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?
        .as_number()
        .ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })
        .map(|n| n as i64)
}

struct AsyncConnectFn {
    socket: Arc<dyn AsyncSocket>,
}

impl AsyncFunction<NonFunctionValue> for AsyncConnectFn {
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        _ctx: &'a mut (dyn Context<NonFunctionValue> + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
        Box::pin(async move {
            let address = require_string_arg(args)?;
            let handle = self.socket.connect(address).await?;
            Ok(NonFunctionValue::new(handle))
        })
    }
}

struct AsyncSendFn {
    socket: Arc<dyn AsyncSocket>,
}

impl AsyncFunction<NonFunctionValue> for AsyncSendFn {
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        _ctx: &'a mut (dyn Context<NonFunctionValue> + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
        Box::pin(async move {
            if args.len() < 2 {
                return Err(Error::ArgumentCount {
                    expected: 2,
                    got: args.len(),
                });
            }
            let handle = require_number_arg(args)?;
            let data = args[1]
                .as_str()
                .ok_or(Error::TypeMismatch {
                    expected: "string",
                    got: "non-string",
                })?
                .to_string();
            let sent = self.socket.send(handle, &data).await?;
            Ok(NonFunctionValue::new(sent))
        })
    }
}

struct AsyncRecvFn {
    socket: Arc<dyn AsyncSocket>,
}

impl AsyncFunction<NonFunctionValue> for AsyncRecvFn {
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        _ctx: &'a mut (dyn Context<NonFunctionValue> + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
        Box::pin(async move {
            if args.len() < 2 {
                return Err(Error::ArgumentCount {
                    expected: 2,
                    got: args.len(),
                });
            }
            let handle = require_number_arg(args)?;
            let length = args[1].as_number().ok_or(Error::TypeMismatch {
                expected: "number",
                got: "non-number",
            })? as usize;
            let data = self.socket.recv(handle, length).await?;
            Ok(NonFunctionValue::new(data))
        })
    }
}

struct AsyncCloseFn {
    socket: Arc<dyn AsyncSocket>,
}

impl AsyncFunction<NonFunctionValue> for AsyncCloseFn {
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        _ctx: &'a mut (dyn Context<NonFunctionValue> + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
        Box::pin(async move {
            let handle = require_number_arg(args)?;
            self.socket.close(handle).await?;
            Ok(NonFunctionValue::default())
        })
    }
}
