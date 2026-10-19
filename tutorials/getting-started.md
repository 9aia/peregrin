# Getting started

```bash
cargo install --path crates/peregrin-cli --bin peregrin
cargo install --path crates/peregrin-cli --bin peregrin-hub
peregrin serve        # node daemon on this machine
peregrin-hub          # hub + web UI
```

The node spawns `sepia serve` for the session plane; the hub aggregates
nodes into one view.
