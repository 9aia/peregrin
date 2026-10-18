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
    run("cargo", &args)
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
                .status()?;
            anyhow::ensure!(status.success(), "workspace tests failed");
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
            let env = [
                ("PEREGRIN_BROWSER_E2E", "1"),
                ("PEREGRIN_E2E_PREBUILT", "1"),
            ];
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
                .envs(env)
                .status()?;
            anyhow::ensure!(status.success(), "browser e2e failed");
        }
        Some(other) => anyhow::bail!("unknown task {other}"),
        None => anyhow::bail!("usage: cargo xtask <check|test|install|site|smoke|e2e>"),
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
