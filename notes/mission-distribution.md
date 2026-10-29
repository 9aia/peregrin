# Mission distribution: install from GitHub

## Problem

Today, `DomainFileRouter` discovers mission files only from the local
filesystem (`.handl/` then the workflow dir). There is no way to share, reuse,
or consume missions authored elsewhere. Every team hand-copies YAML.

## Proposal

GitHub **is** the registry. `org/repo` is the package ID. `handl install`
fetches a repo's `.handl/` once; subsequent runs use the local copy — no
per-run network calls.

```
handl install org/repo                    # default branch
handl install org/repo@v1.2.0             # tag
handl install org/repo@main               # branch
handl install org/repo@<sha>              # exact commit
handl install org/repo --path missions/   # subdirectory instead of .handl/
```

## Install layout

Installed missions live under a separate namespace so local `.handl/` stays
owner-controlled:

```
.handl/
  installed/
    <org>/
      <repo>/
        @<ref>/            # ref + SHA for update tracking
          missions/...
          lib/...
          events/...
  installed.json           # manifest
```

Walk order becomes: local `.handl/` → `.handl/installed/` → workflow dir.
Local wins on name collisions (existing precedence rule still holds).

## Manifest

`.handl/installed.json` tracks every install:

```json
{
  "installs": [
    {
      "id": "org/repo",
      "ref": "main",
      "sha": "abc123",
      "path": ".handl/installed/org/repo/@main",
      "source": "github",
      "installed_at": "2026-08-06T17:30:00Z"
    }
  ]
}
```

## Commands

| Command | Behavior |
|---|---|
| `handl install <ref>` | Fetch `.handl/` (or `--path`) from GitHub, write to `.handl/installed/`, update manifest |
| `handl list` | Show installed missions (id, ref, sha, path) |
| `handl update [id]` | Re-fetch at same ref (or new ref with `@`), diff SHA, replace files |
| `handl remove <id>` | Delete installed dir + manifest entry |

## Fetch strategy

Two options, pick one:

1. **GitHub API** (`GET /repos/{owner}/{repo}/contents/.handl?ref=...`) — no
   git dependency, but paginated and rate-limited (60/hr unauthenticated,
   5000/hr with token). Good for small `.handl/` trees.
2. **Sparse clone** (`git clone --depth 1 --sparse --filter=blob:none`,
   then `sparse-checkout set .handl`) — needs git, but no rate limits and
   handles large trees. `AC_GITHUB_TOKEN` for private repos.

Recommend sparse clone for v1 — simpler, handles private repos, no API
pagination logic. Fall back to API if git is unavailable.

## Conflict policy

- Local `.handl/` files always win over installed (existing precedence).
- Two installed missions with the same node/event/snippet name: last
  installed wins, with a warning. `handl install --force` to override.
- `handl install` refuses if `id` already installed unless `--force` or
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
  manifest and stops `handl update` from moving it.
- **Transitive deps**: does a mission declare dependencies on other
  missions? Defer — no dependency resolution in v1. Missions are flat.
- **Lockfile**: `.handl/installed.lock` with resolved SHAs for reproducible
  installs across machines. Defer to v2.
- **Registry beyond GitHub**: `handl install` could later support git URLs,
  tarball URLs, or a hosted registry. Keep the `<ref>` parser extensible.
