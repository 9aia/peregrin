+++
id = "I-015"
title = "P8 shadow→cutover merge-gated: substrate/pl-003 → v2 merge + live deploy are human calls"
status = "open"
created = "2026-11-04"
priority = "medium"
+++

Everything buildable done (P0-P7,P9 + ci-gate e2e verified). P8 needs: (1) merge substrate/pl-003 into v2 — human/sturn call per no-PR policy; (2) live shadow deploy — webhook registration on real repos + systemd unit enable = external actions needing approval; (3) observation window comparing ci/<campaign> vs ci/bupha. Runbook: import bupha → create → deploy webhook → observe → swap hooks → rollback.
