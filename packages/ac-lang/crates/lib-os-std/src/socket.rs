use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;

use ac_lib_os::{OsError, Socket};

/// Socket implementation backed by [`std::net::TcpStream`].
#[derive(Default)]
pub struct StdSocket {
    next_handle: Mutex<i64>,
    sockets: Mutex<HashMap<i64, TcpStream>>}

impl Socket for StdSocket {
    fn connect(&self, address: &str) -> Result<i64, OsError> {
        let stream = TcpStream::connect(address).map_err(|e| OsError::Io {
            message: e.to_string()})?;
        let mut next = self.next_handle.lock().unwrap();
        let handle = *next;
        *next += 1;
        self.sockets.lock().unwrap().insert(handle, stream);
        Ok(handle)
    }

    fn send(&self, handle: i64, data: &str) -> Result<usize, OsError> {
        let mut sockets = self.sockets.lock().unwrap();
        let stream = sockets.get_mut(&handle).ok_or(OsError::NotFound {
            path: format!("socket handle: {handle}")})?;
        stream.write(data.as_bytes()).map_err(|e| OsError::Io {
            message: e.to_string()})
    }

    fn recv(&self, handle: i64, length: usize) -> Result<String, OsError> {
        let mut sockets = self.sockets.lock().unwrap();
        let stream = sockets.get_mut(&handle).ok_or(OsError::NotFound {
            path: format!("socket handle: {handle}")})?;
        let mut buf = vec![0; length];
        let n = stream.read(&mut buf).map_err(|e| OsError::Io {
            message: e.to_string()})?;
        buf.truncate(n);
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    fn close(&self, handle: i64) -> Result<(), OsError> {
        self.sockets
            .lock()
            .unwrap()
            .remove(&handle)
            .map(|_| ())
            .ok_or(OsError::NotFound {
                path: format!("socket handle: {handle}")})
    }
}
