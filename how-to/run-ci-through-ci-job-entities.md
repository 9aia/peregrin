# Run CI through `ci-job` entities

Production CI ingress is a peregrin campaign: a verified forge webhook
becomes a `kind="ci-job"` entity on the node, and bupha workers claim
and execute it. This guide wires the shipped `ci-ingress` bundle end
to end. For the worker side (fleet config, `tamias-driver-bupha
worker`), see bupha's docs; for running the webhook listener itself,
see [Serve campaigns](/how-to/serve-campaigns).

## The shape

```
github webhook ──► github-webhook node ──► push / pull_request event
                                            │
                          ci.route (typed re-emit)  ◄──┘
                                            ▼
                          ci.enqueue / ci.enqueue.pr   (driver: http)
                                            │  POST /api/entities
                                            ▼
                    peregrin node — CiJobPlane (kind="ci-job")
                                            │
                  tamias-driver-bupha worker ── ci.claim/renew/complete
```

`bupha/libraries/ci-ingress/main.yaml` (the bundle lives in the bupha repo — CI knowledge is bupha's) is a complete campaign: the
`github-webhook` ingress node (HMAC-verified, one `secret_env` per
watched repo), `ci.route` re-emitting typed events
(`{type}.{repo}.{ref}.{clone_url}.{sha}`), and `driver: http`
`ci.enqueue*` nodes that POST the indexDoc to
`${config.node_url}/api/entities`. **Create IS `ci.enqueue`** — the
deterministic entity id `ci:{key}:{sha}:{attempt}` dedups webhook
replays for free, and both enqueue nodes set `retries: 3` since a
re-POST is idempotent.

## Deploy the campaign

The plane lives on the node: `peregrin serve` registers
`bupha_plane::CiJobPlane` behind the default-on `bupha` cargo
feature. Deploy the bundle through tamias — it renders into the
**shared** `peregrin-campaign.service` (every `peregrin-bundle`
declares `provider.name: peregrin`, one listener, one history dir):

```sh
tamias install 9aia/bupha//libraries/ci-ingress   # bundle lives in the bupha repo
tamias deploy ci-ingress                 # → ~/.config/peregrin/serve/ci-ingress.yaml
systemctl --user enable --now peregrin-campaign.service
```

`~/.config/peregrin/env` needs `PEREGRIN_NODE_TOKEN` (bearer for the
node's `/api/entities`) plus one `BUPHA_WH_<OWNER>_<REPO>` HMAC secret
per watched repo — secret **names** only, values never in the
campaign. `config.node_url` in the bundle points the enqueue POST at
the plane's node (`http://127.0.0.1:7700` in the shipped params).

Operator knobs (`params.schema.json`): `node_url`, `queue`
(`latest` = one pending slot per key, newest push wins; `all` =
bounded FIFO, `MAX_PENDING=32`), and `context` (forge status label
prefix, default `ci/bupha`).

## The `ci-job` lifecycle

The indexDoc *is* the job spec — `state`, `fence`,
`leaseExpiresAt`, `claimedBy`, `runsOn`, `queue`, `runner`,
`context`, `maxAttempts` all live on the entity:

- **`pending`** — created by `POST /api/entities` (or requeued by
  `ci.enqueue {attempt?}` on a terminal/superseded entity). A
  spec-armed plane normalizes scheduling fields from the fleet
  config at create and 400s a job whose `ref`/`event` the repo spec
  doesn't cover.
- **`running`** — `ci.claim {worker, leaseSecs?}` CAS
  `pending→running`: sets `claimedBy`, bumps `fence`, stamps
  `leaseExpiresAt`/`startedAt`. A claim that loses the race is a 409.
  The worker renews via `ci.renew` mid-run (default lease 300s,
  `BUPHA_ENTITY_LEASE_SECS`); a dead worker's job returns to
  `pending` when the lease lapses.
- **terminal** — `ci.complete {worker, fence, result}` lands
  `success`/`failure`/`cancelled`/`error` (result `superseded`
  marks a job a newer push replaced — the "latest" lane policy).
  `ci.release` hands a running job back gracefully;
  `ci.cancel` settles pending on the spot or flags a running job
  (`cancelRequested` — the worker's renew loop observes it).
  `ci.status {queuePosition?}` is the rich read.

Workers poll pending entities, claim what their fleet keys + labels
cover, and settle — status posting stays worker-side (forge creds
never leave runner boxes).

**Fork PRs**: `ci.enqueue.pr` also sets the indexDoc's optional
`cloneUrl`/`headRef` from the webhook payload's `head_clone_url` /
head `ref` — populated only when the PR head lives on a fork. The
worker fetches `git fetch <cloneUrl> <headRef>` (then the bare sha)
before falling back to the repo's own remote, so fork heads that
aren't in the base clone still check out. Pushes and same-repo PRs
leave both fields absent.

## Fleet normalization and `runsOn`

Job specs live in **fleet.toml**, not the campaign: `peregrin serve`
loads every visible bupha fleet config — `PEREGRIN_BUPHA_CONFIG` /
`BUPHA_CONFIG` (file or dir) plus
`~/.config/bupha/bundles/*.{toml,yaml,yml}` — and arms the plane with
`CiJobPlane::with_watched`. A `key` matching a fleet spec gets
`queue`/`runsOn`/`runner`/`context`/`maxAttempts` overwritten at
create; unmatched keys keep the indexDoc's values. Repos needing a
per-job lane declare `[[repo.jobs]]` and get their own typed-event
enqueue node (see the shipped `ci.enqueue.job.9aia-bupha-docs`).

`runs_on` on a fleet `[[repo]]`/`[[repo.jobs]]` entry becomes the
entity's `runsOn`. A worker claims only jobs whose `runsOn` ⊆ its
`[worker] labels`; unlabeled jobs are claimable by anyone. The node
re-checks the same gate against its advertised capability labels —
`PEREGRIN_RUNNER_LABELS` on the node is how a bupha worker's labels
ride the same `runs_on` machinery campaign scheduling uses.

## Rehydration after a node restart

The sync direction is node→hub; nothing pushes entities back. At boot,
before the listener binds, the node rebuilds the ci-job plane from
the hub's durable entity index when `PEREGRIN_SNAPSHOT_TARGET` points
at a surviving peer/hub (`PEREGRIN_SNAPSHOT_TOKEN` as bearer — the
same vars the ephemeral shutdown flush uses). Only this node's
bucket is read; an unreachable target logs a warning and the plane
boots empty. This is why a restarted plane doesn't lose pending jobs
or let the hub reconcile-tombstone them.

## Campaigns vs. the entity plane

The ingress is itself a campaign — it rides the shared
`peregrin-campaign.service` listener alongside every other
`peregrin-bundle`, demuxed by repo on the one `--listen/--path`
socket. `kind="ci-job"` entities are a different kind plane on the
same node (`peregrin serve`), not campaign entities — campaigns
schedule via `runs_on` over `/api/node` capabilities, ci-jobs via
`runsOn` over the same label list plus the worker's own
`[worker] labels`. Both reuse one capability-label vocabulary.
