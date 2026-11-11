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
/// SEPIA_TESTKIT_PREBUILT paths. The `peregrin` command itself is
/// gone — the engine e2e spawns `tamias-driver-peregrin run`, the
/// same entrypoint the tamias `bins[]` shim's `cli` verb takes.
const TEST_BINS: &[&str] = &["peregrin-node", "tamias-driver-peregrin"];

fn prebuild() -> anyhow::Result<()> {
    let mut args = vec!["build"];
    for bin in TEST_BINS {
        args.push("--bin");
        args.push(bin);
    }
    run("cargo", &args)?;
    // sepia binaries the suite spawns (drivers, mock agent, the
    // `sepia serve` subprocess) — built once from the sibling checkout
    // (or the cargo git checkout when there is none), then linked into
    // our target/debug so test harnesses that resolve
    // `<root>/target/debug/<bin>` find them uniformly.
    if let Some(root) = sepia_checkout() {
        let manifest = root.join("Cargo.toml");
        run(
            "cargo",
            &[
                "build",
                "--manifest-path",
                manifest.to_str().unwrap_or("../sepia/Cargo.toml"),
                "--bins",
            ],
        )?;
    }
    stage_bins(None)
}

/// Where the sepia workspace sources live for binary building:
/// the sibling `../sepia` checkout when present, else the cargo git
/// checkout the committed ssh:// deps resolve to — sepia-testkit's
/// own `target/debug` fallback points at that checkout's `target/`
/// dir too, so binaries built there are found even unstaged. Prefers
/// the rev pinned in Cargo.lock when several checkouts exist.
fn sepia_checkout() -> Option<std::path::PathBuf> {
    if std::path::Path::new("../sepia/Cargo.toml").exists() {
        return std::path::Path::new("../sepia").canonicalize().ok();
    }
    let checkouts = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)?
        .join(".cargo/git/checkouts");
    // `source = "git+ssh://…/sepia.git?branch=main#<rev>"` — cargo names
    // the checkout dir after the rev's short prefix.
    let lock = std::fs::read_to_string("Cargo.lock").unwrap_or_default();
    let locked: Vec<String> = lock
        .split("sepia.git")
        .skip(1)
        .filter_map(|tail| {
            tail.split('#')
                .nth(1)
                .map(|rev| rev[..7.min(rev.len())].to_owned())
        })
        .collect();
    let mut first: Option<std::path::PathBuf> = None;
    for dir in std::fs::read_dir(checkouts).ok()?.flatten() {
        if !dir
            .file_name()
            .to_str()
            .is_some_and(|n| n.starts_with("sepia-"))
        {
            continue;
        }
        for rev in std::fs::read_dir(dir.path()).ok()?.flatten() {
            let candidate = rev.path();
            if !candidate.join("Cargo.toml").exists() {
                continue;
            }
            if locked.iter().any(|l| {
                rev.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with(l.as_str()))
            }) {
                return Some(candidate);
            }
            first.get_or_insert(candidate);
        }
    }
    first
}

/// Link freshly built workspace + sepia binaries into `target/debug`.
/// `peregrin_dir` overrides where `peregrin*` bins are linked from —
/// the coverage arm passes its isolated target dir so staged links are
/// the instrumented builds, not stale shared-target ones.
fn stage_bins(peregrin_dir: Option<&std::path::Path>) -> anyhow::Result<()> {
    if let Some(sepia_root) = sepia_checkout() {
        // sepia's builds land wherever CARGO_TARGET_DIR points —
        // shared-target first (the global default here), the checkout's
        // repo-local target/debug as fallback.
        let ours = std::path::Path::new("target/debug");
        std::fs::create_dir_all(ours)?;
        for sepia_target in [
            sepia_root.join("target/debug"),
            concat!(env!("HOME"), "/.cargo/shared-target/debug").into(),
        ] {
            let Ok(read) = std::fs::read_dir(&sepia_target) else {
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
    // Our own binaries land wherever CARGO_TARGET_DIR points —
    // link them into repo-local target/debug so harnesses resolving
    // `<root>/target/debug/<bin>` find them under either config. A
    // coverage run passes its isolated target dir first so the
    // instrumented build wins over any stale shared-target binary.
    let mut our_targets: Vec<std::path::PathBuf> = Vec::new();
    if let Some(dir) = peregrin_dir {
        our_targets.push(dir.to_path_buf());
    }
    our_targets.push(concat!(env!("HOME"), "/.cargo/shared-target/debug").into());
    for our_target in our_targets.iter().filter_map(|p| p.to_str()) {
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
    Ok(())
}

fn link_bin(src: &std::path::Path, dst: &std::path::Path) {
    // Relink unconditionally — a staged copy must never shadow a
    // fresher build. Hard links silently break when cargo replaces
    // the source inode, leaving dst frozen at whatever it first held
    // (observed: a stale `target/debug/peregrin` answering spawned-
    // binary tests with last month's verbs).
    if dst.symlink_metadata().is_ok() {
        let _ = std::fs::remove_file(dst);
    }
    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::hard_link(src, dst).or_else(|_| std::fs::copy(src, dst).map(|_| ()));
}

/// `TESTKIT_BIN_DIRS` — where testkit looks for spawned binaries:
/// peregrin's own bins + sepia's (drivers, mock agent, `sepia` itself).
/// `first` prepends a dir — the coverage arm puts its isolated target
/// dir first so tests spawn the *instrumented* peregrin binaries.
fn testkit_bin_dirs(first: Option<&std::path::Path>) -> String {
    let root = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let mut dirs = Vec::new();
    if let Some(first) = first {
        dirs.push(first.display().to_string());
    }
    dirs.extend([
        root.join("target/debug").display().to_string(),
        root.join("../sepia/target/debug").display().to_string(),
        concat!(env!("HOME"), "/.cargo/shared-target/debug").to_string(),
    ]);
    dirs.join(":")
}

/// Captured stdout of a command — for probes like `rustc -vV`.
fn run_out(args: &[&str]) -> Option<String> {
    let (prog, rest) = args.split_first()?;
    let out = Command::new(prog).args(rest).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The host target triple — the `drivers/bin/<triple>/` and
/// `tools/bin/<triple>/` selectors tamias uses for compiled payloads.
fn host_triple() -> String {
    run_out(&["rustc", "-vV"])
        .and_then(|v| {
            v.lines()
                .find_map(|l| l.strip_prefix("host: ").map(str::to_string))
        })
        .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string())
}

/// Release-build `bin` and stage it at `dest` (creating parents,
/// marking it executable on unix).
fn stage_release_bin(bin: &str, dest: &std::path::Path) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(format!("target/release/{bin}"), dest)
        .map_err(|e| anyhow::anyhow!("copy {bin} → {}: {e}", dest.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

/// `pack` — stage the tamias pack payload (T-012): release-build the
/// binaries `libraries/peregrin/tamias.yaml` ships and drop them
/// under the pack's payload dirs —
///
/// - `tamias-driver-peregrin` → `drivers/bin/<host-triple>/` (the
///   bundled driver `bins[].driver: peregrin` resolves),
/// - `peregrin-node`/`peregrin-hub` → `tools/bin/<host-triple>/`
///   (payload bins — real daemon binaries, declared in `bins[]`).
///
/// All gitignored on purpose: the binaries are build artifacts; the
/// committed pack is just `tamias.yaml` + this staging verb.
fn pack() -> anyhow::Result<()> {
    run(
        "cargo",
        &[
            "build",
            "--release",
            "--bin",
            "tamias-driver-peregrin",
            "--bin",
            "peregrin-node",
            "--bin",
            "peregrin-hub",
        ],
    )?;
    let triple = host_triple();
    let pack = std::path::Path::new("libraries/peregrin");
    stage_release_bin(
        "tamias-driver-peregrin",
        &pack
            .join("drivers/bin")
            .join(&triple)
            .join("tamias-driver-peregrin"),
    )?;
    for bin in ["peregrin-node", "peregrin-hub"] {
        stage_release_bin(bin, &pack.join("tools/bin").join(&triple).join(bin))?;
    }
    println!("pack staged under libraries/peregrin/ (triple {triple})");
    println!();
    println!("install the peregrin command (generated bins[] shim):");
    println!("  tamias install --allow-drivers ./libraries/peregrin");
    println!("  # or a pinned tarball: tamias bundle pack libraries/peregrin");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("check") => {
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
                .env("TESTKIT_BIN_DIRS", testkit_bin_dirs(None))
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
            // Isolate coverage artifacts: CARGO_TARGET_DIR is shared
            // (~/.cargo/shared-target) and sibling checkouts' coverage
            // runs drop profraws + instrumented objects into the same
            // llvm-cov-target — foreign files would land in this report
            // at 0% and corrupt TOTAL. A repo-local dir keeps the report
            // to this workspace's objects only.
            let cov_target = std::env::current_dir()?.join("target/llvm-cov-target");
            // Prebuild spawned binaries *inside* the coverage env:
            // llvm-cov's rustc wrapper instruments only the workspace
            // crates named at show-env time — bins built outside it run
            // uninstrumented and e2e coverage stays invisible. Building
            // them into the coverage target dir itself also makes their
            // objects visible to the report (llvm-cov scans
            // <cov-target>/debug), so coverage exercised by spawning
            // `peregrin`/`peregrin-node`/`peregrin-hub` counts — the
            // only way `fn main` and service verbs are reachable.
            // sepia's bins pass through the wrapper uninstrumented (not
            // in this workspace's crate list); sepia measures its own
            // coverage.
            let mut bins = String::new();
            for bin in TEST_BINS {
                bins.push_str(" --bin ");
                bins.push_str(bin);
            }
            let pre = format!(
                "eval \"$(cargo llvm-cov show-env --export-prefix)\" \
                 && cargo build{bins} --target-dir {}",
                cov_target.display()
            );
            let status = Command::new("sh")
                .args(["-c", &pre])
                .env("CARGO_LLVM_COV_TARGET_DIR", &cov_target)
                .status()?;
            anyhow::ensure!(status.success(), "coverage bin prebuild failed");
            if let Some(sepia_root) = sepia_checkout() {
                let sepia_manifest = sepia_root.join("Cargo.toml");
                let build = format!(
                    "eval \"$(cargo llvm-cov show-env --export-prefix)\" \
                     && cargo build --manifest-path {} --bins",
                    sepia_manifest.display()
                );
                let status = Command::new("sh")
                    .args(["-c", &build])
                    .env("CARGO_LLVM_COV_TARGET_DIR", &cov_target)
                    .status()?;
                anyhow::ensure!(status.success(), "sepia bin prebuild failed");
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
            stage_bins(Some(&cov_target.join("debug")))?;
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
                .env(
                    "TESTKIT_BIN_DIRS",
                    testkit_bin_dirs(Some(&cov_target.join("debug"))),
                )
                .env("CARGO_LLVM_COV_TARGET_DIR", &cov_target)
                .status()?;
            anyhow::ensure!(status.success(), "coverage run failed");
        }
        Some("pack") => pack()?,
        Some("install") => {
            // There is no `peregrin` cargo bin anymore — the command
            // ships as the tamias pack in `libraries/peregrin`
            // (`cargo xtask pack` + `tamias install
            // --allow-drivers ./libraries/peregrin`). This installs
            // the driver + daemon binaries themselves so
            // `tamias-driver-peregrin`/`peregrin-hub` resolve on PATH
            // even without the pack.
            let bins = ["peregrin-hub", "tamias-driver-peregrin"];
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
            eprintln!(
                "xtask: the `peregrin` command comes from `cargo xtask pack` + \
                 `tamias install --allow-drivers ./libraries/peregrin`"
            );
        }
        Some(other) => anyhow::bail!("unknown task {other}"),
        None => anyhow::bail!("usage: cargo xtask <check|test|pack|install|coverage>"),
    }
    Ok(())
}
