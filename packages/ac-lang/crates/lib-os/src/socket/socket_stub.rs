//! In-memory [`Socket`] stub for testing.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::OsError;
use crate::socket::Socket;

#[derive(Default)]
struct StubConnection {
    input: Vec<u8>,
    output: Vec<u8>,
}

/// A simple in-memory socket service for tests.
#[derive(Default)]
pub struct StubSocket {
    next_handle: Mutex<i64>,
    sockets: Mutex<HashMap<i64, StubConnection>>,
    bindings: HashMap<String, Vec<u8>>,
}

impl StubSocket {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_binding(address: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(address.into(), data.into());
        StubSocket {
            next_handle: Mutex::new(1),
            sockets: Mutex::new(HashMap::new()),
            bindings,
        }
    }
}

impl Socket for StubSocket {
    fn connect(&self, address: &str) -> Result<i64, OsError> {
        let handle = *self.next_handle.lock().unwrap();
        *self.next_handle.lock().unwrap() += 1;
        let input = self.bindings.get(address).cloned().unwrap_or_default();
        self.sockets.lock().unwrap().insert(
            handle,
            StubConnection {
                input,
                output: Vec::new(),
            },
        );
        Ok(handle)
    }

    fn send(&self, handle: i64, data: &str) -> Result<usize, OsError> {
        let mut sockets = self.sockets.lock().unwrap();
        let conn = sockets.get_mut(&handle).ok_or(OsError::NotFound {
            path: format!("socket handle: {handle}"),
        })?;
        conn.output.extend_from_slice(data.as_bytes());
        Ok(data.len())
    }

    fn recv(&self, handle: i64, length: usize) -> Result<String, OsError> {
        let mut sockets = self.sockets.lock().unwrap();
        let conn = sockets.get_mut(&handle).ok_or(OsError::NotFound {
            path: format!("socket handle: {handle}"),
        })?;
        let end = length.min(conn.input.len());
        let bytes = conn.input.drain(..end).collect::<Vec<_>>();
        Ok(String::from_utf8_lossy(&bytes).into_owned())
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