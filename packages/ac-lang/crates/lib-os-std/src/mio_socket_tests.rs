use ac_runtime_values::NonFunctionValue;
use std::future::Future;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread;

use ac_ast_canonical::Expr;
use ac_lib_os::AsyncSocketModule;
use ac_runtime_canonical::{CanonicalContext, Error, Vm};
use ac_runtime_modules::ModuleHost;

use crate::MioSocket;

fn block_on<F: Future>(future: F) -> F::Output {
    fn noop_clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    fn noop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);

    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = TaskContext::from_waker(&waker);

    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(val) = future.as_mut().poll(&mut cx) {
            return val;
        }
        thread::yield_now();
    }
}

fn make_vm() -> Vm<ModuleHost> {
    let host = ModuleHost::new().with(AsyncSocketModule::new(MioSocket::default()));
    Vm::new(host)
}

#[test]
fn mio_socket_connect_send_recv_close() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let done = Arc::new(AtomicBool::new(false));

    let server_done = done.clone();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).unwrap();
        stream.write_all(&buf[..n]).unwrap();
        server_done.store(true, Ordering::Release);
    });

    let vm = make_vm();
    let mut ctx = CanonicalContext::new();

    let handle = block_on(vm.eval_async(
        &Expr::call(
            "socket.connect",
            vec![Expr::value(NonFunctionValue::new(format!("127.0.0.1:{port}")))],
        ),
        &mut ctx,
    ))
    .unwrap()
    .as_number()
    .unwrap() as i64;

    block_on(vm.eval_async(
        &Expr::call(
            "socket.send",
            vec![
                Expr::value(NonFunctionValue::new(handle)),
                Expr::value(NonFunctionValue::new("hello")),
            ],
        ),
        &mut ctx,
    ))
    .unwrap();

    let received = block_on(vm.eval_async(
        &Expr::call(
            "socket.recv",
            vec![
                Expr::value(NonFunctionValue::new(handle)),
                Expr::value(NonFunctionValue::new(64_i64)),
            ],
        ),
        &mut ctx,
    ))
    .unwrap()
    .as_str()
    .unwrap()
    .to_string();

    assert_eq!(received, "hello");

    block_on(vm.eval_async(
        &Expr::call("socket.close", vec![Expr::value(NonFunctionValue::new(handle))]),
        &mut ctx,
    ))
    .unwrap();

    // Wait for the echo server to finish so the test does not tear down early.
    while !done.load(Ordering::Acquire) {
        thread::yield_now();
    }
    server.join().unwrap();
}

#[test]
fn mio_socket_invalid_address() {
    let vm = make_vm();
    let mut ctx = CanonicalContext::new();

    let err = block_on(vm.eval_async(
        &Expr::call(
            "socket.connect",
            vec![Expr::value(NonFunctionValue::new("not-an-address"))],
        ),
        &mut ctx,
    ))
    .unwrap_err();

    assert!(matches!(err, Error::PrimitiveError(_)));
}

#[test]
fn mio_socket_unknown_handle() {
    let vm = make_vm();
    let mut ctx = CanonicalContext::new();

    let err = block_on(vm.eval_async(
        &Expr::call(
            "socket.send",
            vec![
                Expr::value(NonFunctionValue::new(999_i64)),
                Expr::value(NonFunctionValue::new("x")),
            ],
        ),
        &mut ctx,
    ))
    .unwrap_err();

    assert!(matches!(err, Error::PrimitiveError(_)));
}