# Environment

Every variable peregrin reads. `PEREGRIN_*` configures peregrin itself;
`SEPIA_*` vars are consumed by sepia binaries (the session-plane child
and its drivers) — those are documented in
[sepia's reference](https://9aia.com/sepia/reference/serve-wire).

## Node (`peregrin serve`)

| Variable | Default | Meaning |
| --- | --- | --- |
| `PEREGRIN_HOST` | `127.0.0.1` | Bind address for `/api/*` + SSE |
| `PEREGRIN_PORT` | `8787` | Bind port (legacy `PORT` also read) |
| `PEREGRIN_TOKEN` | — | Bearer auth; **required** on non-loopback binds |
| `PEREGRIN_HOME` | `~/.local/share/peregrin` | Node data dir (identity, meta, tokens) |
| `PEREGRIN_NODE` | `$PEREGRIN_HOME/node.json` | Stable node-identity file |
| `PEREGRIN_NAME` | hostname | Display name for `GET /api/node` |
| `PEREGRIN_META` / `PEREGRIN_META_PATH` | `$PEREGRIN_HOME/meta.json` | Overlay store (pins, archives, spans) |
| `PEREGRIN_ORIGINS` | localhost:3000 ×2 | CORS allowlist; `*` echoes any origin |
| `PEREGRIN_CLINE_DIR` | `~/.cline/data` | Cline store dir — the convert target |
| `PEREGRIN_CLAUDE_DIR` | `~/.claude` | Claude dir — file-history restore |
| `PEREGRIN_SESSION_PLANE` | `inprocess` | `service` spawns `sepia serve` instead |
| `PEREGRIN_SEPIA` | discovery | Explicit path to the `sepia` binary |
| `PEREGRIN_SSE_KEEPALIVE_MS` | `15000` | SSE keep-alive cadence; `0` disables |
| `PEREGRIN_HELD_WATCH_MS` | `5000` | Held-session re-probe cadence |
| `PEREGRIN_IDLE_TTL_MS` | `600000` | Idle live-session detach threshold |
| `PEREGRIN_SWEEP_MS` | `30000` | Idle sweep cadence |
| `PEREGRIN_PAIR_LIMIT_PER_MIN` | `5` | `/api/pair` attempts per minute per client |
| `PEREGRIN_PAIR_LIMIT_PER_DAY` | `20` | `/api/pair` attempts per day per client |
| `PEREGRIN_OTEL_ENDPOINT` | — | OTEL collector endpoint (tracing) |

## Hub (`peregrin-hub`)

| Variable | Default | Meaning |
| --- | --- | --- |
| `PEREGRIN_HUB_HOST` | `127.0.0.1` | Hub bind address |
| `PEREGRIN_HUB_PORT` | `3000` | Hub bind port (`PEREGRIN_PORT` fallback) |
| `PEREGRIN_HUB_TOKEN` | — | Bearer auth for the hub itself |
| `PEREGRIN_NODES` | — | Comma-separated node URLs to aggregate |
| `PEREGRIN_NODE_URL` | — | Single-node shortcut |
| `PEREGRIN_NODE_TOKEN` | — | Token the hub presents to nodes |
| `PEREGRIN_SITE_ROOT` | bundled | Web-bundle root override |
| `PEREGRIN_CREDENTIAL_STORE` | `keyring` | `keyring` or `dotenv` secrets backend |

## CLI (`peregrin`)

| Variable | Meaning |
| --- | --- |
| `PEREGRIN_NODE_URL` | Target node for `peregrin <verb>` commands |
| `PEREGRIN_TOKEN` | Bearer token sent to the node |
