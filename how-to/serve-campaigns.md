# Serve a fleet of campaigns

`peregrin campaign run <yaml>` hosts exactly one campaign in the
foreground — fine for development, but every `github-webhook` ingress
node would want its own listener, so N campaigns collide on
`127.0.0.1:29173`. `peregrin campaign serve` is the multi-campaign
daemon: one process, N campaign definitions, **one shared webhook
listener**.

## Boot

```bash
peregrin campaign serve campaigns/ \
  --listen 127.0.0.1:29173 \
  --path /hook \
  --history-dir ~/.local/share/peregrin/history \
  --queue-store-dir ~/.local/share/peregrin/queues
```

`campaigns/` may be a directory of `*.yaml`/`*.yml` files or any mix
of files and directories — every yaml found becomes one campaign.
Campaign names come from each yaml's `name:` field, falling back to
the file stem; names must be unique or `serve` refuses to boot.

Flags:

- `--listen <addr>` — the shared webhook bind address (default
  `127.0.0.1:29173`). Only meaningful when at least one campaign
  uses `github-webhook`; otherwise no listener binds.
- `--path <path>` — webhook path on that listener (default `/hook`).
- `--history-dir <dir>` — per-campaign JSONL run history at
  `<dir>/<campaign>.jsonl` (same format as `campaign run --history`).
- `--queue-store-dir <dir>` — per-campaign durable supersede queue at
  `<dir>/<campaign>.db` (same sqlite store as
  `campaign run --queue-store`).

Each campaign gets its own `RunSupervisor`, its own supersede policy,
and its own queue/history namespace — a crash or restart of the
process replays every campaign's durable queue independently.

## Shared ingress demux

All `github-webhook` nodes across all campaigns register **routes**
into one shared listener instead of binding their own socket. The
listener is owned by `campaign serve` (`--listen`/`--path`); per-node
`listen`/`path` params are ignored in serve mode.

Routing is **all-match by `repository.full_name`** against each
campaign's `params.repos[].name`:

- A push is delivered to *every* campaign watching that repo.
- If two campaigns watch the same repo with different webhook
  secrets, only the route(s) whose secret validates the
  `X-Hub-Signature-256` receive the event.
- `X-GitHub-Delivery` dedup is global across the listener — a
  redelivery is answered `200 duplicate` once, before fan-out.
- A repo no campaign watches gets `404`, same as the single-campaign
  driver.
- `pull_request` deliveries fan out the same way, but per-route
  action filtering applies: each campaign's
  `params.pull_request_actions` (default `opened,synchronize,reopened`)
  decides which actions it receives — a `synchronize` reaches a route
  that accepts it and is a verified no-op for one that doesn't.
  Hook *registration* is the deploy driver's job — `tamias driver
  exec <pkg> hooks register` subscribes a repo's hook to
  `pull_request` when the manifest declares it; push-only campaigns
  keep a `push`-only subscription. PR payloads
  carry the head repo's `clone_url`, so fork heads are checkable with
  a suitably scoped token — see
  [first-gate](../tutorials/first-gate.md#pull-request-triggers-opt-in).
- A campaign whose run has ended simply drops injections; if every
  matching route is closed the listener answers `503`.

## Shutdown

SIGTERM/SIGINT (or `systemctl stop`) triggers a cooperative drain:
the listener stops accepting, each supervisor finishes its active
run, pending queue entries persist through their `QueueStore`, and
the process exits when every campaign has drained. Active runs are
not killed — they run to completion exactly as `campaign run` would.

## systemd unit

`campaign serve` is a plain long-running process — drop it in a unit:

```ini
# /etc/systemd/system/peregrin-campaigns.service
[Unit]
Description=Peregrin campaign fleet
After=network-online.target

[Service]
Type=simple
User=peregrin
ExecStart=/usr/local/bin/peregrin campaign serve /etc/peregrin/campaigns \
    --listen 127.0.0.1:29173 --path /hook \
    --history-dir /var/lib/peregrin/history \
    --queue-store-dir /var/lib/peregrin/queues
Restart=on-failure
RestartSec=5
# Graceful drain gets up to TimeoutStopSec (default 90s); raise it if
# runs are long.
TimeoutStopSec=300

[Install]
WantedBy=multi-user.target
```

```bash
systemctl daemon-reload
systemctl enable --now peregrin-campaigns
journalctl -u peregrin-campaigns -f   # per-run events print as [<campaign>] ...
```

Host-level service lifecycle is tamias's job — `tamias deploy` of a
`peregrin-bundle` renders exactly this unit shape as the shared
`peregrin-campaign.service` (see
[Distribution](../reference/distribution.md)). Campaign serving is a
workload hosted by that unit; if the `peregrin serve` hub and the
campaign fleet share a host, run them as separate units.
