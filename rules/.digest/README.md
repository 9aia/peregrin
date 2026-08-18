# Rule Digests

Compressed versions of the reference rule files in `rules/`. Each digest is ~30% the size of the original, containing only domain-specific rules as terse bullet points. Universal rules live in `.windsurfrules`. Procedural rules are Devin Skills in `.agents/skills/`. Neither are repeated here.

## System overview

| Layer             | Location                    | Size             | When loaded                      |
| ----------------- | --------------------------- | ---------------- | -------------------------------- |
| Universal rules   | `.windsurfrules`            | ~2KB             | Every conversation               |
| Procedural skills | `.agents/skills/*/SKILL.md` | ~1KB each        | Auto-activated by Devin triggers |
| Reference digests | `rules/.digest/*.md`        | ~30% of original | On-demand, per routing table     |
| Full rules        | `rules/*.md`                | 100%             | Only when digest lacks detail    |

## How it works

1. **Universal rules** (`.windsurfrules`) are always in context.
2. **Procedural skills** (`.agents/skills/`) auto-activate when Devin detects a matching task.
3. **Reference digests** (`rules/.digest/`) are loaded on-demand per the routing table.
4. **Full rules** (`rules/*.md`) are loaded only when a digest lacks sufficient detail.

## Maintenance

Digests are maintained alongside full files. Run `scripts/check-digests.sh` to detect stale digests (source file modified after digest). When a full rule file changes, update its digest to match.
