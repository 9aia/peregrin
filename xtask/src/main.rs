use std::process::{Command, Stdio};

fn run(cmd: &str, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new(cmd).args(args).status()?;
    anyhow::ensure!(status.success(), "{cmd} {args:?} failed");
    Ok(())
}

/// True when the `cargo-nextest` subcommand is installed.
fn nextest_available() -> bool {
    Command::new("cargo")
        .args(["nextest", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// True when the `cargo-llvm-cov` subcommand is installed.
fn llvm_cov_available() -> bool {
    Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Binaries tests shell out to — one prebuild beats N in-test
/// `cargo build`s serializing on locks. `sepia-*` binaries come from
/// the sibling sepia checkout (path deps); tests resolve them via
/// SEPIA_TESTKIT_PREBUILT paths.
const TEST_BINS: &[&str] = &["peregrin", "peregrin-node", "peregrin-hub"];

fn prebuild() -> anyhow::Result<()> {
    let mut args = vec!["build"];
    for bin in TEST_BINS {
        args.push("--bin");
        args.push(bin);
    }
    run("cargo", &args)?;
    // sepia binaries the suite spawns (drivers, mock agent, the
    // `sepia serve` subprocess) — built once from the sibling checkout,
    // then linked into our target/debug so test harnesses that resolve
    // `<root>/target/debug/<bin>` find them uniformly.
    let sepia = std::path::Path::new("../sepia/Cargo.toml");
    if sepia.exists() {
        run(
            "cargo",
            &["build", "--manifest-path", "../sepia/Cargo.toml", "--bins"],
        )?;
    }
    stage_bins_and_site()
}

/// Link freshly built workspace + sepia binaries into `target/debug`
/// and stage the hub's public assets into `target/site`.
fn stage_bins_and_site() -> anyhow::Result<()> {
    if std::path::Path::new("../sepia/Cargo.toml").exists() {
        // sepia's builds land wherever CARGO_TARGET_DIR points —
        // shared-target first (the global default here), repo-local
        // target/debug as fallback.
        let ours = std::path::Path::new("target/debug");
        std::fs::create_dir_all(ours)?;
        for sepia_target in [
            "../sepia/target/debug",
            concat!(env!("HOME"), "/.cargo/shared-target/debug"),
        ] {
            let Ok(read) = std::fs::read_dir(sepia_target) else {
                continue;
            };
            for entry in read.flatten() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                let is_file = entry.file_type().is_ok_and(|t| t.is_file());
                if !(name.starts_with("sepia") && is_file) {
                    continue;
                }
                link_bin(&entry.path(), &ours.join(name));
            }
        }
    }
    // Our own binaries land wherever CARGO_TARGET_DIR points too —
    // link them into repo-local target/debug so harnesses resolving
    // `<root>/target/debug/<bin>` find them under either config.
    for our_target in [concat!(env!("HOME"), "/.cargo/shared-target/debug")] {
        let Ok(read) = std::fs::read_dir(our_target) else {
            continue;
        };
        for entry in read.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let is_file = entry.file_type().is_ok_and(|t| t.is_file());
            if !(name.starts_with("peregrin") && is_file) {
                continue;
            }
            link_bin(
                &entry.path(),
                &std::path::Path::new("target/debug").join(name),
            );
        }
    }
    // e2e harnesses pin PEREGRIN_SITE_ROOT to `target/site` — stage the
    // hub's public assets there (sw.js, manifest, icon) so asset checks
    // pass without a full `xtask site` wasm build. No `pkg/` on purpose:
    // `browser_enabled` keys off it to gate the hydrated-app tests.
    let site = std::path::Path::new("target/site");
    let public = std::path::Path::new("crates/peregrin-hub/public");
    if public.is_dir() {
        let _ = std::fs::create_dir_all(site);
        for entry in std::fs::read_dir(public)?.flatten() {
            let dst = site.join(entry.file_name());
            if entry.file_type().is_ok_and(|t| t.is_file()) && !dst.exists() {
                let _ = std::fs::copy(entry.path(), dst);
            }
        }
    }
    Ok(())
}

fn link_bin(src: &std::path::Path, dst: &std::path::Path) {
    if dst.exists() {
        return;
    }
    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::hard_link(src, dst).or_else(|_| std::fs::copy(src, dst).map(|_| ()));
}

/// `TESTKIT_BIN_DIRS` — where testkit looks for spawned binaries:
/// peregrin's own bins + sepia's (drivers, mock agent, `sepia` itself).
fn testkit_bin_dirs() -> String {
    let root = std::env::current_dir().unwrap_or_else(|_| ".".into());
    format!(
        "{}:{}:{}",
        root.join("target/debug").display(),
        root.join("../sepia/target/debug").display(),
        concat!(env!("HOME"), "/.cargo/shared-target/debug")
    )
}

/// `cargo xtask dev` — the working-tree loop: `cargo leptos watch`
/// rebuilds the wasm bundle + the hub on save and auto-refreshes the
/// browser (the shell's `AutoReload`). Serves on :3002 — the
/// installed prod hub keeps :3000 and the reload websocket :3001.
/// Env comes from `~/.config/peregrin/hub-env` (KEY=value, `#`
/// comments) so the dev hub talks to the same nodes as the unit.
fn dev() -> anyhow::Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(["leptos", "watch"]);
    let env_path = std::env::var_os("HOME").map_or_else(
        || std::path::PathBuf::from(".config/peregrin/hub-env"),
        |h| std::path::PathBuf::from(h).join(".config/peregrin/hub-env"),
    );
    match std::fs::read_to_string(&env_path) {
        Ok(text) => {
            for line in text.lines().map(str::trim) {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    cmd.env(k.trim(), v.trim());
                }
            }
        }
        Err(_) => println!(
            "note: {} not found — running without hub env",
            env_path.display()
        ),
    }
    // A long-running watcher must not sit on the user's flock
    // (~/.cargo.lock serializes cargo invocations) — its children
    // still lock-build normally via cargo's own package-cache lock.
    cmd.env("CARGO_LOCK_HELD", "1");
    cmd.env("PEREGRIN_HUB_PORT", "3002");
    println!("dev hub → http://127.0.0.1:3002 (prod keeps :3000)");
    let status = cmd.status()?;
    anyhow::ensure!(status.success(), "cargo leptos watch failed");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("check") => {
            check_cfg_never_gates_markup()?;
            run("cargo", &["fmt", "--all", "--check"])?;
            run(
                "cargo",
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            )?;
        }
        Some("test") => {
            prebuild()?;
            let cmd = if nextest_available() {
                &["nextest", "run", "--workspace"][..]
            } else {
                &["test", "--workspace"][..]
            };
            let status = Command::new("cargo")
                .args(cmd)
                .env("SEPIA_TESTKIT_PREBUILT", "1")
                .env("PEREGRIN_E2E_PREBUILT", "1")
                .env("TESTKIT_BIN_DIRS", testkit_bin_dirs())
                .status()?;
            anyhow::ensure!(status.success(), "workspace tests failed");
        }
        // `cargo xtask coverage [llvm-cov args…]` — HTML report under
        // target/llvm-cov/html plus a printed summary. Extra args pass
        // through to cargo-llvm-cov (e.g. `--fail-under-lines 100`,
        // `--lcov --output-path lcov.info`); passing any report flag
        // replaces the default --html.
        Some("coverage") => {
            anyhow::ensure!(
                llvm_cov_available(),
                "cargo-llvm-cov not installed — `cargo install cargo-llvm-cov` \
                 (needs the llvm-tools rustup component)"
            );
            // Prebuild spawned binaries *inside* the coverage env:
            // llvm-cov's rustc wrapper instruments only the workspace
            // crates named at show-env time — bins built outside it run
            // uninstrumented and e2e coverage stays invisible. sepia's
            // bins pass through the wrapper uninstrumented (not in this
            // workspace's crate list); sepia measures its own coverage.
            let mut bins = String::new();
            for bin in TEST_BINS {
                bins.push_str(" --bin ");
                bins.push_str(bin);
            }
            let pre = format!(
                "eval \"$(cargo llvm-cov show-env --export-prefix)\" \
                 && cargo build{bins}"
            );
            run("sh", &["-c", &pre])?;
            if std::path::Path::new("../sepia/Cargo.toml").exists() {
                run(
                    "sh",
                    &[
                        "-c",
                        "eval \"$(cargo llvm-cov show-env --export-prefix)\" \
                         && cargo build --manifest-path ../sepia/Cargo.toml \
                         --bins",
                    ],
                )?;
            }
            // Stale non-instrumented links in target/debug would shadow
            // the fresh instrumented builds — drop them before staging.
            if let Ok(read) = std::fs::read_dir("target/debug") {
                for entry in read.flatten() {
                    let name = entry.file_name();
                    let Some(name) = name.to_str() else { continue };
                    let is_file = entry.file_type().is_ok_and(|t| t.is_file());
                    if is_file && (name.starts_with("peregrin") || name.starts_with("sepia")) {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
            stage_bins_and_site()?;
            // --no-tests=pass matches `xtask test`: an empty suite is
            // a pass, not nextest's exit-4 "no tests to run".
            let runner = if nextest_available() {
                "cargo llvm-cov nextest --workspace --exclude xtask --no-tests=pass"
            } else {
                "cargo llvm-cov --workspace --exclude xtask"
            };
            let extra: Vec<String> = std::env::args().skip(2).collect();
            let wants_report = extra.iter().any(|a| {
                matches!(
                    a.as_str(),
                    "--text"
                        | "--summary-only"
                        | "--json"
                        | "--lcov"
                        | "--cobertura"
                        | "--codecov"
                        | "--open"
                )
            });
            let html = if wants_report { "" } else { " --html" };
            let cov = format!("{runner}{html} \"$@\"");
            let status = Command::new("sh")
                .args(["-c", &cov, "xtask-coverage"])
                .args(&extra)
                .env("SEPIA_TESTKIT_PREBUILT", "1")
                .env("PEREGRIN_E2E_PREBUILT", "1")
                .env("TESTKIT_BIN_DIRS", testkit_bin_dirs())
                .status()?;
            anyhow::ensure!(status.success(), "coverage run failed");
        }
        Some("install") => {
            let bins = ["peregrin", "peregrin-hub"];
            let mut build = vec!["build", "--release"];
            for bin in &bins {
                build.push("--bin");
                build.push(*bin);
            }
            run("cargo", &build)?;
            let home = std::env::var_os("HOME")
                .map_or_else(|| std::path::PathBuf::from("/"), std::path::PathBuf::from);
            let out = home.join(".local/bin");
            std::fs::create_dir_all(&out)?;
            for bin in &bins {
                std::fs::copy(format!("target/release/{bin}"), out.join(bin))?;
            }
            println!("installed {bins:?} to {}", out.display());
            build_site()?;
        }
        Some("site") => build_site()?,
        Some("dev") => dev()?,
        // Headless-Chrome console probe against the live stack —
        // the UI regression gate; run after any peregrin-web change.
        Some("smoke") => {
            let args: Vec<String> = std::env::args().skip(2).collect();
            let status = Command::new("python3")
                .arg("tools/browser-smoke.py")
                .args(&args)
                .status()?;
            anyhow::ensure!(status.success(), "browser smoke failed");
        }
        Some("e2e") => {
            prebuild()?;
            anyhow::ensure!(
                std::path::Path::new("target/site/pkg").is_dir(),
                "target/site/pkg missing — run `cargo xtask site` first"
            );
            let status = Command::new("cargo")
                .args([
                    "test",
                    "-p",
                    "peregrin-hub",
                    "--test",
                    "e2e",
                    "--",
                    "--ignored",
                ])
                .env("PEREGRIN_BROWSER_E2E", "1")
                .env("PEREGRIN_E2E_PREBUILT", "1")
                .env("TESTKIT_BIN_DIRS", testkit_bin_dirs())
                .status()?;
            anyhow::ensure!(status.success(), "browser e2e failed");
        }
        Some(other) => anyhow::bail!("unknown task {other}"),
        None => {
            anyhow::bail!("usage: cargo xtask <check|test|install|site|dev|smoke|e2e|coverage>")
        }
    }
    Ok(())
}

/// `cargo xtask site` — build the Leptos wasm bundle + stage the PWA
/// assets into `target/site`, then copy to `~/.local/share/peregrin/site`
/// for `peregrin-hub`'s default site root.
fn build_site() -> anyhow::Result<()> {
    // Tailwind v4 → crates/peregrin-web/style/main.css (include_str!).
    let tw = ["bun", "x", "tailwindcss"];
    let tw_npm = ["npx", "tailwindcss"];
    let (tool, pre) = if which_available(tw[0]) {
        (&tw[..1], &tw[1..])
    } else if which_available(tw_npm[0]) {
        (&tw_npm[..1], &tw_npm[1..])
    } else {
        anyhow::bail!("tailwind css build needs bun or npx on PATH");
    };
    let mut args: Vec<&str> = pre.to_vec();
    args.extend([
        "-i",
        "crates/peregrin-web/style/input.css",
        "-o",
        "crates/peregrin-web/style/main.css",
        "--minify",
    ]);
    run(tool[0], &args)?;

    let site = std::path::Path::new("target/site");
    std::fs::create_dir_all(site.join("pkg"))?;
    run(
        "cargo",
        &[
            "build",
            "--release",
            "--package",
            "peregrin-web",
            "--target",
            "wasm32-unknown-unknown",
            "--features",
            "hydrate",
        ],
    )?;
    run(
        "wasm-bindgen",
        &[
            "--target",
            "web",
            "--out-dir",
            "target/site/pkg",
            "target/wasm32-unknown-unknown/release/peregrin_web.wasm",
        ],
    )?;
    // Content-hash the bundle stem so stale caches can't hydrate old
    // wasm against new SSR HTML.
    let wasm = site.join("pkg/peregrin_web_bg.wasm");
    let hash = {
        use sha2::Digest;
        let bytes = std::fs::read(&wasm)?;
        sha2::Sha256::digest(&bytes)[..4]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let stem = format!("peregrin_web_{hash}");
    let js = site.join("pkg/peregrin_web.js");
    std::fs::write(
        site.join(format!("pkg/{stem}.js")),
        std::fs::read_to_string(&js)?.replace("peregrin_web_bg.wasm", &format!("{stem}_bg.wasm")),
    )?;
    std::fs::rename(&wasm, site.join(format!("pkg/{stem}_bg.wasm")))?;
    std::fs::remove_file(&js)?;
    for file in ["manifest.json", "sw.js", "icon.svg"] {
        let src = format!("crates/peregrin-hub/public/{file}");
        if std::path::Path::new(&src).exists() {
            std::fs::copy(&src, site.join(file))?;
        }
    }
    let home = std::env::var_os("PEREGRIN_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".local/share/peregrin"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".peregrin"));
    let dst = home.join("site");
    let _ = std::fs::remove_dir_all(dst.join("pkg"));
    std::fs::create_dir_all(&dst)?;
    copy_dir(site, &dst)?;
    println!("site staged to {}", dst.display());
    Ok(())
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// `check` gate: `#[cfg]` may gate *behavior*, never markup shape — a
/// cfg'd `view!`/`impl IntoView` pair is exactly the divergence class
/// behind the ThemeToggle hydration panic.
fn check_cfg_never_gates_markup() -> anyhow::Result<()> {
    const LOOKAHEAD: usize = 8;
    let mut files = Vec::new();
    rs_files(std::path::Path::new("crates/peregrin-web/src"), &mut files)?;
    let mut violations = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim_start();
            if !(t.starts_with("#[cfg(")
                && (t.contains(r#"feature = "ssr""#) || t.contains(r#"feature = "hydrate""#)))
            {
                continue;
            }
            for (j, next) in lines.iter().enumerate().skip(i + 1).take(LOOKAHEAD) {
                let n = next.trim_start();
                if n.starts_with("//") {
                    continue;
                }
                if n.contains("view!") || (n.contains("impl ") && n.contains("IntoView")) {
                    violations.push(format!(
                        "  {}:{} — `{t}` → `{}` (line {})",
                        path.display(),
                        i + 1,
                        n,
                        j + 1
                    ));
                    break;
                }
            }
        }
    }
    anyhow::ensure!(
        violations.is_empty(),
        "cfg must gate behavior, never markup shape:\n{}",
        violations.join("\n")
    );
    Ok(())
}

fn rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            rs_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn which_available(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
