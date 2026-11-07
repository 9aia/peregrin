+++
id = "I-016"
title = "peregrin campaign run/validate + agent/shell drivers exist only on the v2 branch"
status = "done"
created = "2026-11-03"
owner = "devin"
priority = "medium"
labels = ["campaign", "peregrin"]
+++

The adversarial campaign (campaigns/adversarial/main.yaml) targets peregrin's campaign engine, which is fully ported only on the peregrin repo's `v2` branch: `peregrin campaign run|validate`, drivers (shell/agent/http/human), loader-yaml include/snippet expansion. On `main` those crates are stubs and the installed `peregrin` binary (0.0.0) has no campaign verb — `peregrin campaign run` errors 'unrecognized subcommand'. The campaign validates + runs only against a v2 build (cargo build -p peregrin-cli on branch v2). Also deferred in the port: ${config.*}/${state.*}/${event.*} scope lookups pass through unresolved (campaign routes state via files instead), and the shell driver hardcodes a 30 s timeout (slow steps must be agent nodes). The other integration gap: arca.toml.example shows `[serve.pdp] exec = "peregrin policy decide"` as the intended PDP — no such peregrin verb exists yet; campaigns/adversarial/pdp-campaign.sh is the stand-in.

## Resolution

Stale — campaign run/validate and agent/shell/git/http/human drivers all landed on main (driver crates + peregrin_node::campaign::registry); new drivers (schedule, subworkflow, council) build on them.
