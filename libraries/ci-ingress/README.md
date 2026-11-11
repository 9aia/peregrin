# ci-ingress

Forge webhook → `kind="ci-job"` enqueue — the production ingress half
of the bupha CI plane (P-002 §4, ADR-0007; bupha ledger T-058/T-063).
A verified `push` becomes `POST /api/entities {kind:"ci-job"}` on the
peregrin node hosting [`CiJobPlane`](https://github.com/9aia/bupha)
— **create IS `ci.enqueue`**, and the deterministic entity id
`ci:{key}:{sha}:{attempt}` dedups webhook replays for free.

```
github webhook ──► github-webhook node ──► push event
                                            │
            ci.route (typed re-emit)  ◄─────┤
                                            ▼
                              ci.enqueue (driver: http)
                                            │  POST /api/entities
                                            ▼
                          peregrin-node /api/entities  (kind="ci-job")
                                            │
                          tamias-driver-bupha worker ── ci.claim/renew/complete
```

## Pieces

- **`main.yaml`** — the campaign: `github-webhook` ingress (bupha's
  port/path conventions), `ci.route` typed re-emit
  (`push.{repo}.{ref}.{clone_url}.{sha}`, ci-gate's pattern), and
  `ci.enqueue`, a `driver: http` node that POSTs the indexDoc.
- **The plane** — `peregrin-node`'s default-on `bupha` cargo feature
  registers `bupha_plane::CiJobPlane` next to `CampaignEntityPlane`
  (`crates/peregrin-node/src/lib.rs`). `runsOn` matching reads the
  same capability labels campaign `runs_on` does — a worker's
  `[worker] labels` ride `PEREGRIN_RUNNER_LABELS`.
- **The worker** — `tamias-driver-bupha worker` polls
  `kind="ci-job"`, `ci.claim`s what its fleet.toml `[[repo]] keys` +
  labels cover, runs the sha, `ci.complete`s. Status posting stays
  worker-side (forge creds never leave runner boxes).

## Deploy

The node hosting the plane must run a `peregrin` build with the
`bupha` feature (default). Then:

```sh
tamias install ./libraries/ci-ingress
tamias deploy ci-ingress        # → shared peregrin-campaign.service
systemctl --user enable --now peregrin-campaign.service
```

`~/.config/peregrin/env` needs:

- `PEREGRIN_NODE_TOKEN` — bearer for the entity node's `/api/entities`
  (the node's `PEREGRIN_TOKEN`, or a paired credential).
- `BUPHA_WH_<OWNER>_<REPO>` — one HMAC secret per watched repo
  (same names ci-gate/docs-deploy use; extend `repos[]` +
  `capabilities.secrets` per deployment).

`config.node_url` points the enqueue POST at the plane's node —
loopback when the campaign daemon and `peregrin serve` share a host.

## Per-repo job specs

The shipped `ci.enqueue` is the generic catch-all: `key` = repo
`full_name`, `queue`/`context` from `config:`. Repos needing their
own lane policy, `owner/name:job` key suffix, `runsOn` labels,
`maxAttempts`, or runner hints get an extra node on the typed event:

```yaml
- id: ci.enqueue.myrepo-gate
  driver: http
  on:
  - push.myorg/myrepo.refs/heads/main.**.*
  max_fires: unlimited
  emits: [http.done, http.failed]
  params:
    url: "${config.node_url}/api/entities"
    method: POST
    headers:
      authorization: "Bearer ${secret.PEREGRIN_NODE_TOKEN}"
    body:
      kind: ci-job
      indexDoc:
        key: "myorg/myrepo:gate"        # must equal the worker's [[repo]] key
        repo: "myorg/myrepo"
        ref: "${event.payload.ref}"
        sha: "${event.payload.sha}"
        event: push
        sender: "${event.payload.actor}"
        queue: "all"                    # bounded FIFO for this repo
        runsOn: ["arch=arm64"]
        context: "ci/bupha/gate"
        maxAttempts: 3                  # literal — templates stringify
```

Note `maxAttempts`/`runsOn` must be YAML literals — `${config.*}`
substitutions always render strings, and the plane's indexDoc
validation is typed (`u32`/`[string]`). `queue`/`context` are safe
via `config:` (strings either way).

The worker resolves execution (runner kind, workdir, status sink)
from **its own** fleet.toml `[[repo]]` entry matched by `key` — the
indexDoc's `runner`/`runsOn` are the plane's scheduling record, not
the run spec. Keep `key` spellings identical on both sides.

## Not covered yet (filed remainders)

- **`pull_request` deliveries** — the webhook driver emits
  `pull_request` events; this bundle only subscribes `push`. PR
  lanes need `event: "pull_request"` + a PR-scoped key convention.
- **Per-repo spec resolution** — bupha's `Config::watched` →
  indexDoc normalization (queue/runsOn/runner/context/maxAttempts
  resolved per `[[repo]]`) is not exposed to campaigns; today the
  bundle emits repo-key jobs with fleet defaults and per-repo
  overrides are hand-written nodes (above). A `ci-ingress`
  normalization driver is the follow-up.
- **`deliveryId`** — the push payload doesn't carry
  `X-GitHub-Delivery`; the deterministic entity id already dedups
  replays, so it's audit-only when it lands.
- **Enqueue retry** — the `http` POST is fire-and-forget: a down
  entity node surfaces `http.failed` in the campaign log but
  nothing replays it (the plane's idempotent create makes a forge
  redelivery safe if one is configured upstream).
