//! Default search-path branches: `./peregrin.*`,
//! `~/.config/peregrin/peregrin.*`, and the final miss.
//!
//! `HOME` and cwd are process-global, and edition-2024 makes
//! `env::set_var` `unsafe` (forbidden workspace-wide) — so each
//! scenario runs in a re-exec'd child whose env (`HOME`) and cwd the
//! parent's `Command` controls. The parent writes the fixtures and
//! spawns one child per scenario; the child asserts.
#![allow(clippy::unwrap_used)]

use peregrin_config::ConfigSource;
use peregrin_config_file::FileSource;
use std::path::Path;

fn load_default() -> Result<peregrin_core::Campaign, String> {
    FileSource { path: None }.load()
}

/// Child-side scenario assertions. cwd and `HOME` are already the
/// scenario dirs (set by the spawning `Command`).
fn run_scenario(n: &str) {
    match n {
        // `./peregrin.toml` alone serves.
        "0" => assert_eq!(load_default().unwrap().name, "s0-toml"),
        // Within `./`, yaml > yml > toml.
        "1" => {
            assert_eq!(load_default().unwrap().name, "yaml");
            std::fs::remove_file("peregrin.yaml").unwrap();
            assert_eq!(load_default().unwrap().name, "yml");
            std::fs::remove_file("peregrin.yml").unwrap();
            assert_eq!(load_default().unwrap().name, "toml");
        }
        // Empty `./` falls through to `~/.config/peregrin/`.
        "2" => assert_eq!(load_default().unwrap().name, "home-yaml"),
        // Step order beats format order: `./peregrin.toml` wins over
        // `~/.config/peregrin/peregrin.yaml`.
        "3" => assert_eq!(load_default().unwrap().name, "s3-toml"),
        // Nothing anywhere -> the miss error.
        "4" => {
            let e = load_default().unwrap_err();
            assert!(e.contains("no peregrin.{yaml,yml,toml}"), "{e}");
        }
        other => panic!("unknown scenario {other}"),
    }
}

/// Write `scenario/{cwd,home}` fixtures for scenario `n` under `base`.
fn fixture(base: &Path, n: usize) -> (std::path::PathBuf, std::path::PathBuf) {
    let cwd = base.join(format!("s{n}/cwd"));
    let home = base.join(format!("s{n}/home"));
    let cfgd = home.join(".config/peregrin");
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(&cfgd).unwrap();
    match n {
        0 => {
            std::fs::write(cwd.join("peregrin.toml"), "name = \"s0-toml\"\n").unwrap();
        }
        1 => {
            std::fs::write(cwd.join("peregrin.yaml"), "name: yaml\n").unwrap();
            std::fs::write(cwd.join("peregrin.yml"), "name: yml\n").unwrap();
            std::fs::write(cwd.join("peregrin.toml"), "name = \"toml\"\n").unwrap();
        }
        2 => {
            std::fs::write(cfgd.join("peregrin.yaml"), "name: home-yaml\n").unwrap();
        }
        3 => {
            std::fs::write(cwd.join("peregrin.toml"), "name = \"s3-toml\"\n").unwrap();
            std::fs::write(cfgd.join("peregrin.yaml"), "name: home-yaml\n").unwrap();
        }
        4 => {}
        other => panic!("unknown scenario {other}"),
    }
    (cwd, home)
}

#[test]
fn search_paths_in_order() {
    // Child: run the scenario the parent selected.
    if let Ok(n) = std::env::var("PERE_FILE_SCENARIO") {
        run_scenario(&n);
        return;
    }
    // Parent: fixtures + one re-exec'd child per scenario.
    let base = tempfile::TempDir::new().unwrap();
    let test = std::thread::current().name().unwrap().to_string();
    for n in 0..=4 {
        let (cwd, home) = fixture(base.path(), n);
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([test.as_str(), "--exact", "--nocapture"])
            .env("PERE_FILE_SCENARIO", n.to_string())
            .env("HOME", &home)
            .env_remove("USERPROFILE")
            .current_dir(&cwd)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "scenario {n} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
