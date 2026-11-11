//! Driver conformance gate — runs pavo's wire-contract suite
//! ([`pavo_testkit::conformance::check_driver`]) against this
//! crate's own `tamias-driver-peregrin serve` binary.
//!
//! `manifest`/`views`/`query:"ops"` never touch peregrin state;
//! `cli.*` ops are not exercised by the suite, so no node or
//! `PEREGRIN_HOME` is needed — hermetic, no network.

use std::path::Path;

#[test]
fn driver_passes_pavo_conformance() {
    let violations = pavo_testkit::conformance::check_driver(Path::new(env!(
        "CARGO_BIN_EXE_tamias-driver-peregrin"
    )));
    assert!(
        violations.is_empty(),
        "pavo conformance violations:\n{}",
        violations
            .iter()
            .map(|v| format!("  - {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
