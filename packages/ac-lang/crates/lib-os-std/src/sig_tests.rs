use ac_lib_os::Sig;

use crate::StdSig;

#[test]
fn std_sig_list_contains_common_signals() {
    let sig = StdSig::default();
    let list = sig.list().unwrap();
    assert!(list.contains(&"SIGINT".into()));
    assert!(list.contains(&"SIGTERM".into()));
}

#[test]
fn std_sig_send_invalid_signal() {
    let sig = StdSig::default();
    let result = sig.send(1, "BOGUS");
    assert!(result.is_err());
}

#[test]
fn std_sig_send_invalid_process_id() {
    let sig = StdSig::default();
    let result = sig.send(0, "SIGTERM");
    assert!(result.is_err());
}