#![allow(unknown_lints, clippy::assert_is_empty)]
#![allow(clippy::unwrap_used)]
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use peregrin_driver::{Driver, ExecutionContext};
use peregrin_driver_human::{HumanDriver, Prompter};
use peregrin_template::{Effects, SystemEffects};
use serde_json::{Value, json};

fn node(params: Value) -> peregrin_core::Node {
    peregrin_core::Node {
        id: "n".into(),
        driver: "human".into(),
        params,
        ..Default::default()
    }
}

/// A scripted prompter — records the question, answers with a canned
/// line (or fails/sleeps per scenario).
struct FakePrompter {
    answer: Result<String, String>,
    delay: Option<Duration>,
    seen: mpsc::Sender<String>,
}

impl Prompter for FakePrompter {
    fn question(&self, prompt: &str) -> Result<String, String> {
        let _ = self.seen.send(prompt.to_string());
        if let Some(delay) = self.delay {
            thread::sleep(delay);
        }
        self.answer.clone()
    }
}

fn fake(answer: &str) -> (HumanDriver, mpsc::Receiver<String>) {
    let (tx, rx) = mpsc::channel();
    (
        HumanDriver::with_prompter(Arc::new(FakePrompter {
            answer: Ok(answer.to_string()),
            delay: None,
            seen: tx,
        })),
        rx,
    )
}

/// A prompter that dies mid-question — disconnects the answer channel.
struct Panicky;

impl Prompter for Panicky {
    fn question(&self, _prompt: &str) -> Result<String, String> {
        panic!("boom");
    }
}

/// cwd/read_file under test control — covers the `process.cwd()`-joined
/// `prompt_file` branch without touching the process cwd.
struct DirEffects {
    root: PathBuf,
}

impl Effects for DirEffects {
    fn env(&self, name: &str) -> Option<String> {
        SystemEffects.env(name)
    }
    fn cwd(&self) -> PathBuf {
        self.root.clone()
    }
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| e.to_string())
    }
    fn shell(&self, command: &str) -> Result<String, String> {
        SystemEffects.shell(command)
    }
    fn now_unix(&self) -> i64 {
        0
    }
    fn now_iso(&self) -> String {
        String::new()
    }
    fn random(&self) -> f64 {
        0.5
    }
    fn uuid(&self) -> String {
        "u".to_string()
    }
}

#[test]
fn name_and_missing_prompt() {
    let (driver, _rx) = fake("y");
    let _ = HumanDriver::default();
    assert_eq!(driver.name(), "human");
    let ctx = ExecutionContext::default();
    let err = driver.invoke(&node(json!({})), &[], &ctx).unwrap_err();
    assert_eq!(err.reason, "human node 'n' missing 'params.prompt'");
}

#[test]
fn dry_run() {
    let (driver, _rx) = fake("unused");
    let dry = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let out = driver
        .invoke(&node(json!({"prompt": "ok?"})), &[], &dry)
        .unwrap();
    assert_eq!(out[0].kind, "n.ok");
    assert_eq!(
        out[0].payload,
        json!({"response": "(dry-run)", "ok": true, "timed_out": false})
    );

    // emit_ok overrides the dry-run kind too.
    let out = driver
        .invoke(
            &node(json!({"prompt": "ok?", "emit_ok": "go.ahead"})),
            &[],
            &dry,
        )
        .unwrap();
    assert_eq!(out[0].kind, "go.ahead");
}

#[test]
fn answer_trims_and_routes() {
    let ctx = ExecutionContext::default();

    // Non-empty answer → ok on emits[0].
    let (driver, rx) = fake("  yes  \n");
    let mut n = node(json!({"prompt": "Proceed?"}));
    n.emits = vec!["a.ok".into(), "a.no".into()];
    let out = driver.invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "a.ok");
    assert_eq!(
        out[0].payload,
        json!({"response": "yes", "ok": true, "timed_out": false})
    );
    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), "Proceed?");

    // Empty answer → reject on emits[1].
    let (driver, _rx) = fake("\n");
    let out = driver.invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "a.no");
    assert_eq!(out[0].payload["ok"], false);

    // Reject with a lone emits entry falls to emits.last.
    let mut n = node(json!({"prompt": "Proceed?"}));
    n.emits = vec!["only".into()];
    let out = driver.invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "only");

    // No emits at all → the {id}.rejected fallback.
    let out = driver
        .invoke(&node(json!({"prompt": "Proceed?"})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].kind, "n.rejected");

    // emit_reject overrides everything.
    let out = driver
        .invoke(
            &node(json!({"prompt": "Proceed?", "emit_reject": "nope.out"})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "nope.out");
}

#[test]
fn expect_matching() {
    let ctx = ExecutionContext::default();

    let (driver, _rx) = fake("yes");
    let out = driver
        .invoke(&node(json!({"prompt": "?", "expect": "yes"})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].payload["ok"], true);
    assert_eq!(out[0].kind, "n.ok");

    let (driver, _rx) = fake("no");
    let out = driver
        .invoke(&node(json!({"prompt": "?", "expect": "yes"})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].payload["ok"], false);
    assert_eq!(out[0].kind, "n.rejected");
}

#[test]
fn timeout_marks_timed_out() {
    let ctx = ExecutionContext::default();
    let (tx, _rx) = mpsc::channel();
    // The prompter blocks past the 50 ms deadline — the abort arm wins.
    let driver = HumanDriver::with_prompter(Arc::new(FakePrompter {
        answer: Ok("late".to_string()),
        delay: Some(Duration::from_millis(400)),
        seen: tx,
    }));
    let out = driver
        .invoke(
            &node(json!({"prompt": "?", "timeout": 0.05, "expect": "late"})),
            &[],
            &ctx,
        )
        .unwrap();
    // Timeout loses even to a matching `expect`.
    assert_eq!(out[0].kind, "n.rejected");
    assert_eq!(
        out[0].payload,
        json!({"response": "", "ok": false, "timed_out": true})
    );
}

#[test]
fn timeout_coercion_variants() {
    let ctx = ExecutionContext::default();
    let (driver, _rx) = fake("ok");

    // Numeric string coerces like JS Number().
    let out = driver
        .invoke(&node(json!({"prompt": "?", "timeout": "30"})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].payload["ok"], true);

    // Non-numeric → NaN → no timeout (the question still answers).
    let out = driver
        .invoke(&node(json!({"prompt": "?", "timeout": "soon"})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].payload["ok"], true);

    // null → `?? 300` default; true → 1s; [2] → 2s; {} → NaN.
    for timeout in [Value::Null, json!(true), json!([2]), json!({})] {
        let out = driver
            .invoke(&node(json!({"prompt": "?", "timeout": timeout})), &[], &ctx)
            .unwrap();
        assert_eq!(out[0].payload["ok"], true);
    }

    // 0 → explicit no timeout.
    let out = driver
        .invoke(&node(json!({"prompt": "?", "timeout": 0})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].payload["ok"], true);
}

#[test]
fn prompt_file_prepends() {
    let ctx = ExecutionContext::default();

    // Absolute path — contents + "\n" + prompt reach the prompter.
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "context line").unwrap();
    let (driver, rx) = fake("ok");
    driver
        .invoke(
            &node(json!({"prompt": "the question", "prompt_file": file.path()})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        "context line\nthe question"
    );

    // Relative path joins against effects.cwd().
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ctx.md"), "relative ctx").unwrap();
    let (tx, rx) = mpsc::channel();
    let driver = HumanDriver::with_parts(
        Arc::new(DirEffects {
            root: dir.path().to_path_buf(),
        }),
        Arc::new(FakePrompter {
            answer: Ok("ok".to_string()),
            delay: None,
            seen: tx,
        }),
    );
    driver
        .invoke(
            &node(json!({"prompt": "?", "prompt_file": "ctx.md"})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        "relative ctx\n?"
    );

    // Missing file → DriverError.
    let (driver, _rx) = fake("ok");
    let err = driver
        .invoke(
            &node(json!({"prompt": "?", "prompt_file": "/nonexistent/f.md"})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(!err.reason.is_empty());

    // Empty file → prompt stays untouched.
    let file = tempfile::NamedTempFile::new().unwrap();
    let (driver, rx) = fake("ok");
    driver
        .invoke(
            &node(json!({"prompt": "plain", "prompt_file": file.path()})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), "plain");
}

#[test]
fn prompter_failures() {
    let ctx = ExecutionContext::default();
    let (tx, _rx) = mpsc::channel();

    // The question effect failing is a DriverError (with a deadline —
    // the `Ok(Err)` arm).
    let driver = HumanDriver::with_prompter(Arc::new(FakePrompter {
        answer: Err("stdin closed".to_string()),
        delay: None,
        seen: tx,
    }));
    let err = driver
        .invoke(&node(json!({"prompt": "?", "timeout": 5})), &[], &ctx)
        .unwrap_err();
    assert_eq!(err.reason, "stdin closed");

    // Without a deadline the failure propagates identically.
    let (tx, _rx) = mpsc::channel();
    let driver = HumanDriver::with_prompter(Arc::new(FakePrompter {
        answer: Err("stdin closed".to_string()),
        delay: None,
        seen: tx,
    }));
    let err = driver
        .invoke(&node(json!({"prompt": "?", "timeout": 0})), &[], &ctx)
        .unwrap_err();
    assert_eq!(err.reason, "stdin closed");

    // A panicking prompter disconnects the channel → DriverError.
    let driver = HumanDriver::with_prompter(Arc::new(Panicky));
    let err = driver
        .invoke(&node(json!({"prompt": "?", "timeout": 5})), &[], &ctx)
        .unwrap_err();
    assert!(err.reason.contains("prompter thread died"));
}

/// The stdin body can't run headless — `cfg(coverage)` swaps in the
/// probe, which this test (itself coverage-only) exercises.
#[cfg(coverage)]
#[test]
fn stdin_prompter_probe() {
    use peregrin_driver_human::StdinPrompter;
    assert_eq!(StdinPrompter.question("?").unwrap(), "");
}
