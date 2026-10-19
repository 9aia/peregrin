#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! Arbitration decisions + the lock registry — pure policy, no I/O.

use std::time::Duration;

use peregrin_control::{Actor, Arbitration, Decision, LockRegistry, NodeId, Ownership, Route};

fn actor(id: &str, node: &str) -> Actor {
    Actor {
        id: id.into(),
        node: NodeId(node.into()),
    }
}

#[test]
fn free_session_allows_rw_attach() {
    let reg = LockRegistry::default();
    let policy = Arbitration::default();
    assert_eq!(
        policy.attach(&reg, "s1", &actor("cli", "n1"), false),
        Decision::Allow
    );
}

#[test]
fn same_holder_re_attaches_freely() {
    let mut reg = LockRegistry::default();
    reg.grant("s1", actor("web", "n1"));
    assert_eq!(
        Arbitration::default().attach(&reg, "s1", &actor("web", "n1"), false),
        Decision::Allow
    );
}

#[test]
fn held_session_attaches_read_only_without_takeover() {
    let mut reg = LockRegistry::default();
    reg.grant("s1", actor("web", "n1"));
    assert_eq!(
        Arbitration::default().attach(&reg, "s1", &actor("cli", "n1"), false),
        Decision::AttachReadOnly {
            holder: "web".into()
        }
    );
}

#[test]
fn takeover_preempts_a_live_holder() {
    let mut reg = LockRegistry::default();
    reg.grant("s1", actor("web", "n1"));
    assert_eq!(
        Arbitration::default().attach(&reg, "s1", &actor("cli", "n1"), true),
        Decision::Preempt {
            stale_holder: "web".into()
        }
    );
}

#[test]
fn stale_lock_is_preemptable_without_takeover() {
    let mut reg = LockRegistry::default();
    reg.grant("s1", actor("web", "n1"));
    // Zero staleness bound → any lock is immediately preemptable.
    let policy = Arbitration {
        stale_after: Duration::ZERO,
        ..Arbitration::default()
    };
    assert_eq!(
        policy.attach(&reg, "s1", &actor("cli", "n1"), false),
        Decision::Preempt {
            stale_holder: "web".into()
        }
    );
}

#[test]
fn strict_mode_queues_when_contended() {
    let mut reg = LockRegistry::default();
    reg.grant("s1", actor("web", "n2"));
    let policy = Arbitration {
        read_only_on_contention: false,
        ..Arbitration::default()
    };
    assert_eq!(
        policy.attach(&reg, "s1", &actor("cli", "n1"), false),
        Decision::Queue {
            node: NodeId("n2".into())
        }
    );
}

#[test]
fn refresh_keeps_a_lock_from_going_stale() {
    let mut reg = LockRegistry::default();
    reg.grant("s1", actor("web", "n1"));
    std::thread::sleep(Duration::from_millis(10));
    reg.refresh("s1", "web");
    assert!(reg.stale(Duration::from_secs(60)).is_empty());
    reg.release("s1", "web");
    assert!(reg.holder("s1").is_none());
}

#[test]
fn routing_local_live_offline_and_unknown() {
    let mut own = Ownership::default();
    let local = NodeId("n1".into());
    own.index("s1", NodeId("n1".into()));
    own.index("s2", NodeId("n2".into()));
    own.index("s3", NodeId("n3".into()));
    own.set_live(NodeId("n2".into()), true);
    own.set_live(NodeId("n3".into()), false);
    assert_eq!(own.route("s1", &local), Route::Local);
    assert_eq!(own.route("s2", &local), Route::Node(NodeId("n2".into())));
    assert_eq!(own.route("s3", &local), Route::Offline(NodeId("n3".into())));
    assert_eq!(own.route("s4", &local), Route::Unknown);
    assert_eq!(
        own.sessions_on(&NodeId("n2".into())),
        vec!["s2".to_string()]
    );
}
