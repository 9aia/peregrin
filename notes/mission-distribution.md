# Mission distribution: install from GitHub

## Problem

Today, `DomainFileRouter` discovers mission files only from the local
filesystem (`.ac/` then the workflow dir). There is no way to share, reuse,
or consume missions authored elsewhere. Every team hand-copies YAML.

## Proposal

GitHub **is** the registry. `org/repo` is the package ID. `ac install`
fetches a repo's `.ac/` once; subsequent runs use the local copy — no
per-run network calls.

```
ac install org/repo                    # default branch
ac install org/repo@v1.2.0             # tag
ac install org/repo@main               # branch
ac install org/repo@<sha>              # exact commit
ac install org/repo --path missions/   # subdirectory instead of .ac/
```

## Install layout

Installed missions live under a separate namespace so local `.ac/` stays
owner-controlled:

```
.ac/
  installed/
    <org>/
      <repo>/
        @<ref>/            # ref + SHA for update tracking
          missions/...
          lib/...
          events/...
  installed.json           # manifest
```

Walk order becomes: local `.ac/` → `.ac/installed/` → workflow dir.
Local wins on name collisions (existing precedence rule still holds).

## Manifest

`.ac/installed.json` tracks every install:

```json
{
  "installs": [
    {
      "id": "org/repo",
      "ref": "main",
      "sha": "abc123",
      "path": ".ac/installed/org/repo/@main",
      "source": "github",
      "installed_at": "2026-08-06T17:30:00Z"
    }
  ]
}
```

## Commands

| Command | Behavior |
|---|---|
| `ac install <ref>` | Fetch `.ac/` (or `--path`) from GitHub, write to `.ac/installed/`, update manifest |
| `ac list` | Show installed missions (id, ref, sha, path) |
| `ac update [id]` | Re-fetch at same ref (or new ref with `@`), diff SHA, replace files |
| `ac remove <id>` | Delete installed dir + manifest entry |

## Fetch strategy

Two options, pick one:

1. **GitHub API** (`GET /repos/{owner}/{repo}/contents/.ac?ref=...`) — no
   git dependency, but paginated and rate-limited (60/hr unauthenticated,
   5000/hr with token). Good for small `.ac/` trees.
2. **Sparse clone** (`git clone --depth 1 --sparse --filter=blob:none`,
   then `sparse-checkout set .ac`) — needs git, but no rate limits and
   handles large trees. `AC_GITHUB_TOKEN` for private repos.

Recommend sparse clone for v1 — simpler, handles private repos, no API
pagination logic. Fall back to API if git is unavailable.

## Conflict policy

- Local `.ac/` files always win over installed (existing precedence).
- Two installed missions with the same node/event/snippet name: last
  installed wins, with a warning. `ac install --force` to override.
- `ac install` refuses if `id` already installed unless `--force` or
  `--update`.

## Caching

No runtime cache needed — installed files are on disk. The only network
call is at `install`/`update` time, which is explicit. Runs stay local.

## Auth

- Public repos: no token needed (sparse clone) or unauthenticated API.
- Private repos: `AC_GITHUB_TOKEN` env var or `gh auth` integration.
- Token is read at install time only, never persisted in the manifest.

## Open questions

- **Versioning**: pin by SHA for reproducibility, or float on a branch?
  Suggest: default to branch HEAD, `--pin` writes the resolved SHA to
  manifest and stops `ac update` from moving it.
- **Transitive deps**: does a mission declare dependencies on other
  missions? Defer — no dependency resolution in v1. Missions are flat.
- **Lockfile**: `.ac/installed.lock` with resolved SHAs for reproducible
  installs across machines. Defer to v2.
- **Registry beyond GitHub**: `ac install` could later support git URLs,
  tarball URLs, or a hosted registry. Keep the `<ref>` parser extensible.
