use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Mutex;
use std::time::Duration;

use handl_lib_os::{OsError, Socket};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Socket implementation backed by [`std::net::TcpStream`].
#[derive(Default)]
pub struct StdSocket {
    next_handle: Mutex<i64>,
    sockets: Mutex<HashMap<i64, TcpStream>>,
}

impl Socket for StdSocket {
    fn connect(&self, address: &str) -> Result<i64, OsError> {
        let addrs: Vec<_> = address
            .to_socket_addrs()
            .map_err(|e| OsError::InvalidArgument {
                message: format!("invalid address: {e}"),
            })?
            .collect();
        let addr = addrs.first().ok_or(OsError::InvalidArgument {
            message: "invalid address: no addresses found".into(),
        })?;
        let stream =
            TcpStream::connect_timeout(addr, DEFAULT_TIMEOUT).map_err(io_error_to_os_error)?;
        stream
            .set_read_timeout(Some(DEFAULT_TIMEOUT))
            .map_err(io_error_to_os_error)?;
        stream
            .set_write_timeout(Some(DEFAULT_TIMEOUT))
            .map_err(io_error_to_os_error)?;
        let mut next = self.next_handle.lock().unwrap();
        let handle = *next;
        *next += 1;
        self.sockets.lock().unwrap().insert(handle, stream);
        Ok(handle)
    }

    fn send(&self, handle: i64, data: &str) -> Result<usize, OsError> {
        let mut sockets = self.sockets.lock().unwrap();
        let stream = sockets.get_mut(&handle).ok_or(OsError::NotFound {
            path: format!("socket handle: {handle}"),
        })?;
        stream.write(data.as_bytes()).map_err(io_error_to_os_error)
    }

    fn recv(&self, handle: i64, length: usize) -> Result<String, OsError> {
        let mut sockets = self.sockets.lock().unwrap();
        let stream = sockets.get_mut(&handle).ok_or(OsError::NotFound {
            path: format!("socket handle: {handle}"),
        })?;
        let mut buf = vec![0; length];
        let n = stream.read(&mut buf).map_err(io_error_to_os_error)?;
        buf.truncate(n);
        String::from_utf8(buf).map_err(|_| OsError::InvalidArgument {
            message: "received non-UTF-8 data".into(),
        })
    }

    fn close(&self, handle: i64) -> Result<(), OsError> {
        self.sockets
            .lock()
            .unwrap()
            .remove(&handle)
            .map(|_| ())
            .ok_or(OsError::NotFound {
                path: format!("socket handle: {handle}"),
            })
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
