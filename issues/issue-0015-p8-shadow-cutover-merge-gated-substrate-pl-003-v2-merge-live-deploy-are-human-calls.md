+++
id = "I-015"
title = "P8 shadow→cutover merge-gated: substrate/pl-003 → v2 merge + live deploy are human calls"
status = "done"
created = "2026-11-04"
priority = "medium"
+++

Everything buildable done (P0-P7,P9 + ci-gate e2e verified). P8 needs: (1) merge substrate/pl-003 into v2 — human/sturn call per no-PR policy; (2) live shadow deploy — webhook registration on real repos + systemd unit enable = external actions needing approval; (3) observation window comparing ci/<campaign> vs ci/bupha. Runbook: import bupha → create → deploy webhook → observe → swap hooks → rollback.

## Resolution

Cutover done 2026-11-09: substrate merged into main (as PL-003 ancestry), ci-gate.service owns :29173 serving imported 188-repo config, tamias.luisfuturist.com tunnel live, all 188 hooks retargeted, bupha stopped. Fixes shipped: BUPHA_TOKEN->gh token, git bearer->Basic (main).
