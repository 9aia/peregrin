//! `peregrin service` verbs against a bogus-named unit — real systemd
//! user bus, zero risk to real units (`peregrin-covtest` never
//! existed). On hosts without `systemctl` the verbs degrade to
//! `CliError`s, which is still coverage — assertions stay permissive.
#![allow(clippy::unwrap_used)]
#![cfg(unix)]

use peregrin_cli::service::{self, ServiceSpec, launchd, systemd};
use tempfile::TempDir;

fn spec(name: &str, env: &std::path::Path) -> ServiceSpec {
    ServiceSpec {
        name: name.into(),
        exec: vec!["/bin/true".into()],
        env_file: env.to_path_buf(),
        system: false,
    }
}

#[test]
fn backend_dispatch_and_mod_verbs() {
    let b = service::detect_backend().unwrap();
    assert_eq!(b.id(), "systemd");
    let dir = TempDir::new().unwrap();
    let s = spec("peregrin-covtest-dispatch", &dir.path().join("env"));
    let _ = b.unit_path(&s);
    let _ = b.install(&s);
    let _ = b.status(&s);
    let _ = b.restart(&s);
    let _ = b.enable(&s);
    let _ = b.disable(&s);
    let _ = b.logs(&s, false);
    let _ = b.uninstall(&s, true);
    // The mod-level verbs: only the read-only ones are host-safe —
    // status + logs against a name that doesn't exist.
    let _ = service::service_status(true, false);
    let _ = service::service_logs(true, false, false);
    let _ = service::default_env_file(std::path::Path::new("/h"), "peregrin");
    let _ = service::default_env_file(std::path::Path::new("/h"), "peregrin-hub");
    assert!(service::env_template().contains("PEREGRIN_NODES"));
}

#[test]
fn systemd_lifecycle() {
    if !std::path::Path::new("/usr/sbin/systemctl").exists()
        && !std::path::Path::new("/bin/systemctl").exists()
    {
        return; // no systemd on this host — nothing to exercise
    }
    let dir = TempDir::new().unwrap();
    let s = spec("peregrin-covtest", &dir.path().join("env"));
    let unit = systemd::unit_path(&s);
    let _ = std::fs::remove_file(&unit); // clean slate
    // status on an unknown unit → NOT_INSTALLED shape.
    let st = systemd::status(&s).unwrap();
    assert!(!st.installed);
    // install writes the unit + env template and tries enable --now
    // (may fail if ExecStart can't run — either way the verb ran).
    let _ = systemd::install(&s);
    assert!(unit.exists());
    assert!(s.env_file.exists());
    // status now sees an installed unit → the full probe path.
    let st = systemd::status(&s).unwrap();
    assert!(st.installed);
    let _ = systemd::restart(&s);
    let _ = systemd::enable(&s);
    let _ = systemd::disable(&s);
    let code = systemd::logs(&s, false).unwrap();
    assert!(code == 0 || code == 1);
    systemd::uninstall(&s, true).unwrap();
    assert!(!unit.exists());
    // uninstall tolerates a missing unit.
    let _ = systemd::uninstall(&s, false);
}

#[test]
fn launchd_safe_paths() {
    let dir = TempDir::new().unwrap();
    let s = spec("covtest", &dir.path().join("env"));
    // No launchctl on linux — verbs resolve to clear errors/empties;
    // uninstall is best-effort filesystem and always lands.
    let _ = launchd::install(&s);
    let _ = launchd::status(&s);
    let _ = launchd::restart(&s);
    let _ = launchd::enable(&s);
    let _ = launchd::disable(&s);
    let _ = launchd::logs(&s, false);
    launchd::uninstall(&s, true).unwrap();
    let _ = launchd::unit_path(&s);
    assert!(launchd::render(&s).contains("<plist"));
}
