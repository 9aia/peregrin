# TODO

## Rename: campaign → mission

The unit of work is now called a **mission** (long-running, goal-oriented,
self-authoring, audited event-graph). "Campaign" carried marketing baggage
and didn't signal the differentiator. README prose already uses "mission";
this is the full rename of identifiers and paths.

- Learn from last campaign (ndocs3-export-architecture-improvements), e.g. build-in harness-agnostic agent, harness abstraction to be able to resume by id and continue with prompts or convert into other harness sessions, builtin agent that fixes workflows on demand selfhealing)
- [ ] Repo/project name: `agent-campaign` → `agent-mission` (or similar)
- [ ] Python package: `agent_campaign/` → `agent_mission/`
- [ ] CLI: `handl` → `am` (or keep `handl`? decide)
- [ ] Directory convention: `campaigns/` → `missions/`
- [ ] Example paths in docs: `campaigns/my-campaign/` → `missions/my-mission/`
- [ ] `.handl/` runtime dir → `.am/` (or keep `.handl/`? decide — breaking change for existing deployments)
- [ ] `pyproject.toml`, `egg-info`, imports, tests
- [ ] Docs (`docs/`, README, examples) — all prose references
- [ ] ndocs `.handl/` deployments — migration plan

## Mission distribution: install from GitHub

GitHub as registry, `handl install org/repo` fetches `.handl/` once, runs stay local.

- [ ] `handl install <gh-ref>` — sparse clone of `.handl/` into `.handl/installed/`
- [ ] `handl list` / `handl update` / `handl remove`
- [ ] Manifest (`.handl/installed.json`) tracking ref + SHA + path
- [ ] Walk order: local `.handl/` → `.handl/installed/` → workflow dir
- [ ] Auth: `AC_GITHUB_TOKEN` for private repos
- [ ] Conflict policy (local wins, `--force` to override)

Details: [notes/mission-distribution.md](notes/mission-distribution.md)

## Distributed mode

See [plans/distributed.md](../plans/distributed.md) for the full design.

- [ ] Phase 1: Redis bus + store (same machine, multiple processes)
- [ ] Phase 2: Node placement + work stealing
- [ ] Phase 3: Leader election + failure recovery
- [ ] Phase 4: NATS + Kafka backends
