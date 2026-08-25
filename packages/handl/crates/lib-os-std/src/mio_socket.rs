use std::collections::HashMap;
use std::future::Future;
use std::io::{self, Read, Write};
use std::net::{Shutdown, ToSocketAddrs};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

use mio::net::TcpStream;
use mio::{Events, Interest, Poll as MioPoll, Token};

use handl_lib_os::{AsyncSocket, OsError};

/// Async socket implementation backed by `mio` on Unix.
#[derive(Default)]
pub struct MioSocket {
    next_handle: Mutex<i64>,
    sockets: Mutex<HashMap<i64, Arc<Mutex<TcpStream>>>>,
}

impl AsyncSocket for MioSocket {
    fn connect<'a>(
        &'a self,
        address: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<i64, OsError>> + Send + 'a>> {
        Box::pin(async move {
            let mut addrs = address
                .to_socket_addrs()
                .map_err(|e| OsError::InvalidArgument {
                    message: format!("invalid address: {e}"),
                })?;
            let addr = addrs.next().ok_or(OsError::InvalidArgument {
                message: "invalid address: no addresses found".into(),
            })?;

            let stream = TcpStream::connect(addr).map_err(io_error_to_os_error)?;
            let conn = Arc::new(Mutex::new(stream));
            let handle = {
                let mut n = self.next_handle.lock().unwrap();
                let h = *n;
                *n += 1;
                h
            };
            self.sockets.lock().unwrap().insert(handle, conn.clone());

            WaitForReady::new(conn.clone(), Interest::WRITABLE)
                .await
                .map_err(io_error_to_os_error)?;

            {
                let guard = conn.lock().unwrap();
                if let Some(err) = guard.take_error().map_err(io_error_to_os_error)? {
                    return Err(io_error_to_os_error(err));
                }
                match guard.peer_addr() {
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::NotConnected => {
                        return Err(io_error_to_os_error(e));
                    }
                    Err(e) => return Err(io_error_to_os_error(e)),
                }
            }

            Ok(handle)
        })
    }

    fn send<'a>(
        &'a self,
        handle: i64,
        data: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<usize, OsError>> + Send + 'a>> {
        Box::pin(async move {
            let conn = {
                let guard = self.sockets.lock().unwrap();
                guard
                    .get(&handle)
                    .ok_or(OsError::NotFound {
                        path: format!("socket handle: {handle}"),
                    })?
                    .clone()
            };

            WaitForReady::new(conn.clone(), Interest::WRITABLE)
                .await
                .map_err(io_error_to_os_error)?;

            let n = {
                let mut guard = conn.lock().unwrap();
                (&mut *guard)
                    .write(data.as_bytes())
                    .map_err(io_error_to_os_error)?
            };

            Ok(n)
        })
    }

    fn recv<'a>(
        &'a self,
        handle: i64,
        length: usize,
    ) -> Pin<Box<dyn Future<Output = Result<String, OsError>> + Send + 'a>> {
        Box::pin(async move {
            let conn = {
                let guard = self.sockets.lock().unwrap();
                guard
                    .get(&handle)
                    .ok_or(OsError::NotFound {
                        path: format!("socket handle: {handle}"),
                    })?
                    .clone()
            };

            WaitForReady::new(conn.clone(), Interest::READABLE)
                .await
                .map_err(io_error_to_os_error)?;

            let mut buf = vec![0u8; length];
            let n = {
                let mut guard = conn.lock().unwrap();
                (&mut *guard).read(&mut buf).map_err(io_error_to_os_error)?
            };
            buf.truncate(n);

            String::from_utf8(buf).map_err(|_| OsError::InvalidArgument {
                message: "received non-UTF-8 data".into(),
            })
        })
    }

    fn close<'a>(
        &'a self,
        handle: i64,
    ) -> Pin<Box<dyn Future<Output = Result<(), OsError>> + Send + 'a>> {
        Box::pin(async move {
            let conn = {
                let mut guard = self.sockets.lock().unwrap();
                guard.remove(&handle).ok_or(OsError::NotFound {
                    path: format!("socket handle: {handle}"),
                })?
            };
            let _ = conn.lock().unwrap().shutdown(Shutdown::Both);
            Ok(())
        })
    }
}

/// Waits for a `mio` readiness event for a single connection, waking the
/// `std::task::Waker` from a background `mio::Poll` loop.
struct WaitForReady {
    conn: Arc<Mutex<TcpStream>>,
    interest: Interest,
    ready: Arc<AtomicBool>,
    error: Arc<Mutex<Option<io::Error>>>,
    started: bool,
}

impl WaitForReady {
    fn new(conn: Arc<Mutex<TcpStream>>, interest: Interest) -> Self {
        WaitForReady {
            conn,
            interest,
            ready: Arc::new(AtomicBool::new(false)),
            error: Arc::new(Mutex::new(None)),
            started: false,
        }
    }
}

impl Future for WaitForReady {
    type Output = Result<(), io::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.ready.load(Ordering::Acquire) {
            let err = self.error.lock().unwrap().take();
            return match err {
                Some(e) => Poll::Ready(Err(e)),
                None => Poll::Ready(Ok(())),
            };
        }

        if !self.started {
            self.started = true;
            let waker: Waker = cx.waker().clone();
            let ready = self.ready.clone();
            let error = self.error.clone();
            let conn = self.conn.clone();
            let interest = self.interest;

            thread::spawn(move || {
                let mut poll = match MioPoll::new() {
                    Ok(p) => p,
                    Err(e) => {
                        *error.lock().unwrap() = Some(e);
                        ready.store(true, Ordering::Release);
                        waker.wake();
                        return;
                    }
                };
                let mut events = Events::with_capacity(4);

                {
                    let mut guard = conn.lock().unwrap();
                    if let Err(e) = poll.registry().register(&mut *guard, Token(0), interest) {
                        *error.lock().unwrap() = Some(e);
                        ready.store(true, Ordering::Release);
                        waker.wake();
                        return;
                    }
                }

                let poll_err = poll.poll(&mut events, Some(Duration::from_secs(5))).err();

                // Deregister the stream before dropping the `Poll` so the
                // `TcpStream` can be re-registered by a later operation.
                {
                    let mut guard = conn.lock().unwrap();
                    let _ = poll.registry().deregister(&mut *guard);
                }
                drop(poll);

                if let Some(e) = poll_err {
                    *error.lock().unwrap() = Some(e);
                } else if events.is_empty() {
                    *error.lock().unwrap() = Some(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "mio poll timed out waiting for socket readiness",
                    ));
                }

                ready.store(true, Ordering::Release);
                waker.wake();
            });
        }

        Poll::Pending
    }
}

fn io_error_to_os_error(error: io::Error) -> OsError {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => OsError::PermissionDenied {
            path: error.to_string(),
        },
        _ => OsError::Io {
            message: error.to_string(),
        },
    }
}
