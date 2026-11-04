//! `peregrin-mcp` — the MCP stdio server binary. Agents connect over
//! stdio (newline-delimited JSON-RPC); every tool is a thin wrapper
//! over the library composition the `peregrin` CLI drives. The body
//! blocks on stdin, so under `cfg(coverage)` it compiles to a no-op —
//! `peregrin_mcp::serve_io` is what tests drive.

#[cfg(not(coverage))]
fn main() -> std::io::Result<()> {
    peregrin_mcp::serve(&peregrin_mcp::Ctx::default())
}

/// Coverage-build placeholder — the real body blocks on stdin.
#[cfg(coverage)]
fn main() {}
