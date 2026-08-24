use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use ac_lib_os::{OsError, Socket};

use crate::StdSocket;

#[test]
fn std_socket_connect_send_recv_close() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).unwrap();
        stream.write_all(&buf[..n]).unwrap();
    });

    let socket = StdSocket::default();
    let handle = socket.connect(&format!("127.0.0.1:{port}")).unwrap();
    socket.send(handle, "hello").unwrap();
    let received = socket.recv(handle, 64).unwrap();
    assert_eq!(received, "hello");
    socket.close(handle).unwrap();
    server.join().unwrap();
}

#[test]
fn std_socket_invalid_address() {
    let socket = StdSocket::default();
    let err = socket.connect("not-an-address").unwrap_err();
    assert!(matches!(err, OsError::InvalidArgument { .. }));
}

#[test]
fn std_socket_unknown_handle() {
    let socket = StdSocket::default();
    let err = socket.send(999, "x").unwrap_err();
    assert!(matches!(err, OsError::NotFound { .. }));
}

#[test]
fn std_socket_recv_rejects_non_utf8() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(&[0xff, 0xfe]).unwrap();
    });

    let socket = StdSocket::default();
    let handle = socket.connect(&format!("127.0.0.1:{port}")).unwrap();
    let err = socket.recv(handle, 64).unwrap_err();
    assert!(matches!(err, OsError::InvalidArgument { .. }));
    server.join().unwrap();
}