# Security Policy

Report vulnerabilities privately to the maintainers via GitHub:
https://github.com/9aia/peregrin/security/advisories/new

Do not open public issues for security reports.

## Scope

- `peregrin serve` binds the `/api/*` surface — non-loopback binds
  refuse to start without `PEREGRIN_TOKEN` (bearer auth); `/api/pair`
  is rate-limited (`PEREGRIN_PAIR_LIMIT_PER_MIN`/`_PER_DAY`).
- `github-webhook` ingress nodes verify `X-Hub-Signature-256` HMAC
  against the per-repo secret *name* (`PEREGRIN_WH_<OWNER>_<REPO>`)
  before a delivery becomes an event; unverified payloads never
  reach the campaign engine. `X-GitHub-Delivery` replays dedup.
- Secrets resolve **by name** through env → OS keyring →
  `~/.config/peregrin/env` (chmod 600) — values never sit in
  campaign YAML, node config, or event payloads; the
  `capabilities.secrets` allow-list gates which names a node may
  resolve.
- Bundle installs are digest-pinned: `tamias verify <name>`
  recomputes the payload digest; `git:`/`http(s)` refs record the
  resolved sha. `capabilities:` policy (secrets, agents, urls,
  shell/http/python/webhook allow-deny) bounds what a node may do —
  see `docs/explanation/safety.md` and `threat-model.md`.

## Notes

Campaign node configs are operator-owned (`~/.config/peregrin/`) —
a deployed bundle never ships `params.repos`/`secret_env` values.
The event envelope is closed (five fields) so run logs stay
shareable. See `docs/reference/distribution.md` for the
install/deploy boundary and `project/adrs/` for design history.

We respond to reports as quickly as we can; expect acknowledgement
within a few days.
