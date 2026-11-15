# Command-Line Help for `peregrin`

This document contains the help content for the `peregrin` command-line program.

Drive a peregrin node over its API, or convert sessions between the Devin, Cline, Claude and Cursor stores

**Command Overview:**

* [`peregrin`↴](#peregrin)
* [`peregrin health`↴](#peregrin-health)
* [`peregrin node`↴](#peregrin-node)
* [`peregrin agents`↴](#peregrin-agents)
* [`peregrin user`↴](#peregrin-user)
* [`peregrin fs`↴](#peregrin-fs)
* [`peregrin events`↴](#peregrin-events)
* [`peregrin redeem`↴](#peregrin-redeem)
* [`peregrin prompt`↴](#peregrin-prompt)
* [`peregrin sessions`↴](#peregrin-sessions)
* [`peregrin sessions list`↴](#peregrin-sessions-list)
* [`peregrin sessions create`↴](#peregrin-sessions-create)
* [`peregrin sessions attach`↴](#peregrin-sessions-attach)
* [`peregrin sessions prompt`↴](#peregrin-sessions-prompt)
* [`peregrin sessions cancel`↴](#peregrin-sessions-cancel)
* [`peregrin sessions permission`↴](#peregrin-sessions-permission)
* [`peregrin sessions history`↴](#peregrin-sessions-history)
* [`peregrin sessions checkpoints`↴](#peregrin-sessions-checkpoints)
* [`peregrin sessions export`↴](#peregrin-sessions-export)
* [`peregrin sessions stream`↴](#peregrin-sessions-stream)
* [`peregrin sessions meta`↴](#peregrin-sessions-meta)
* [`peregrin sessions rename`↴](#peregrin-sessions-rename)
* [`peregrin sessions delete`↴](#peregrin-sessions-delete)
* [`peregrin sessions convert`↴](#peregrin-sessions-convert)
* [`peregrin sessions import`↴](#peregrin-sessions-import)
* [`peregrin sessions resume`↴](#peregrin-sessions-resume)
* [`peregrin sessions restore`↴](#peregrin-sessions-restore)
* [`peregrin sessions rewind`↴](#peregrin-sessions-rewind)
* [`peregrin campaign`↴](#peregrin-campaign)
* [`peregrin campaign run`↴](#peregrin-campaign-run)
* [`peregrin campaign validate`↴](#peregrin-campaign-validate)
* [`peregrin campaign submit`↴](#peregrin-campaign-submit)
* [`peregrin campaign status`↴](#peregrin-campaign-status)
* [`peregrin campaign events`↴](#peregrin-campaign-events)
* [`peregrin campaign serve`↴](#peregrin-campaign-serve)
* [`peregrin bundle`↴](#peregrin-bundle)
* [`peregrin bundle install`↴](#peregrin-bundle-install)
* [`peregrin bundle remove`↴](#peregrin-bundle-remove)
* [`peregrin bundle list`↴](#peregrin-bundle-list)
* [`peregrin bundle info`↴](#peregrin-bundle-info)
* [`peregrin logs`↴](#peregrin-logs)
* [`peregrin logs runs`↴](#peregrin-logs-runs)
* [`peregrin logs events`↴](#peregrin-logs-events)
* [`peregrin logs explain`↴](#peregrin-logs-explain)
* [`peregrin projects`↴](#peregrin-projects)
* [`peregrin projects list`↴](#peregrin-projects-list)
* [`peregrin projects create`↴](#peregrin-projects-create)
* [`peregrin projects init`↴](#peregrin-projects-init)
* [`peregrin projects rename`↴](#peregrin-projects-rename)
* [`peregrin projects delete`↴](#peregrin-projects-delete)
* [`peregrin config`↴](#peregrin-config)
* [`peregrin config get`↴](#peregrin-config-get)
* [`peregrin config set`↴](#peregrin-config-set)
* [`peregrin push`↴](#peregrin-push)
* [`peregrin push vapid`↴](#peregrin-push-vapid)
* [`peregrin push subscribe`↴](#peregrin-push-subscribe)
* [`peregrin push unsubscribe`↴](#peregrin-push-unsubscribe)
* [`peregrin pair`↴](#peregrin-pair)
* [`peregrin serve`↴](#peregrin-serve)
* [`peregrin import`↴](#peregrin-import)
* [`peregrin import bupha`↴](#peregrin-import-bupha)
* [`peregrin runs`↴](#peregrin-runs)
* [`peregrin log`↴](#peregrin-log)
* [`peregrin explain`↴](#peregrin-explain)
* [`peregrin version`↴](#peregrin-version)

## `peregrin`

Drive a peregrin node over its API, or convert sessions between the Devin, Cline, Claude and Cursor stores

**Usage:** `peregrin <COMMAND>`

###### **Commands:**

* `health` — GET /api/health — is the node up and its store readable
* `node` — GET /api/node — the node's identity, agents and capabilities
* `agents` — GET /api/agents — the agent runtimes registered on the node
* `user` — GET /api/user — the OS user the node runs as
* `fs` — GET /api/fs — list subdirectories of a path on the node
* `events` — GET /api/events — stream the node event feed (session/meta/project/heartbeat) to stdout
* `redeem` — POST /api/pair — exchange a `peregrin pair` code for a long-lived bearer token
* `prompt` — POST /api/sessions/:id/prompt — send one turn to an attached session
* `sessions` — Node ops against /api/sessions — run against a live peregrin node
* `campaign` — Campaign ops — load and run campaign yamls on this machine's campaign plane (run, validate, plus the submit+poll agent surface)
* `bundle` — Installed-bundle store ops (install/remove/list/info) — the tamias install pipeline under peregrin's store/cache roots
* `logs` — JSONL run-log observability — scan dirs, tail events, explain runs (bounded; the live feed stays `events`)
* `projects` — Node-local project grouping (/api/projects)
* `config` — Config ops — the node's server-side config (get/set)
* `push` — Web-push subscription management (/api/push/*)
* `pair` — Print a one-time pairing code — the UI exchanges it via POST /api/pair
* `serve` — Serve the peregrin node — the `/api/*` surface (PEREGRIN_* env configures it)
* `import` — Foreign config → peregrin node-config workflow conversion (ADR-0081 bupha importer)
* `runs` — Recent runs — per-push records read back from the durable JSONL history (newest first)
* `log` — Raw event log — the last N lines of the JSONL history
* `explain` — Why a sha did/didn't run — the run's events + outcome, or the leftover worktree it died in
* `version` — Print the peregrin version stamp

###### **Options:**

* `-h, --help` — Print help
* `-V, --version` — Print version

## `peregrin health`

GET /api/health — is the node up and its store readable

**Usage:** `peregrin health [OPTIONS]`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin node`

GET /api/node — the node's identity, agents and capabilities

**Usage:** `peregrin node [OPTIONS]`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin agents`

GET /api/agents — the agent runtimes registered on the node

**Usage:** `peregrin agents [OPTIONS]`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin user`

GET /api/user — the OS user the node runs as

**Usage:** `peregrin user [OPTIONS]`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin fs`

GET /api/fs — list subdirectories of a path on the node

**Usage:** `peregrin fs [OPTIONS] <PATH>`

###### **Arguments:**

* `<PATH>` — Absolute directory path on the node

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin events`

GET /api/events — stream the node event feed (session/meta/project/heartbeat) to stdout

**Usage:** `peregrin events [OPTIONS]`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin redeem`

POST /api/pair — exchange a `peregrin pair` code for a long-lived bearer token

**Usage:** `peregrin redeem [OPTIONS] <CODE>`

###### **Arguments:**

* `<CODE>` — One-time code printed by `peregrin pair` on the node

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `-h, --help` — Print help

## `peregrin prompt`

POST /api/sessions/:id/prompt — send one turn to an attached session

**Usage:** `peregrin prompt [OPTIONS] <SESSION_ID> <TEXT>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node
* `<TEXT>` — The prompt text to send

###### **Options:**

* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions`

Node ops against /api/sessions — run against a live peregrin node

**Usage:** `peregrin sessions <COMMAND>`

###### **Commands:**

* `list` — GET /api/sessions — list the node's sessions
* `create` — POST /api/sessions — create a session
* `attach` — POST /api/sessions/:id/attach — attach live control (read-only while another process holds the lock)
* `prompt` — POST /api/sessions/:id/prompt — send one turn to an attached session
* `cancel` — POST /api/sessions/:id/cancel — stop the current run
* `permission` — POST /api/sessions/:id/permission — answer a pending permission request
* `history` — GET /api/sessions/:id/history — the paginated backlog
* `checkpoints` — GET /api/sessions/:id/checkpoints — workspace snapshot refs the store recorded
* `export` — GET /api/sessions/:id/export — the complete session IR (feeds `sessions import`)
* `stream` — GET /api/sessions/:id/stream — stream the live run's session events (sepia-proto SessionEvent JSON) to stdout
* `meta` — PATCH /api/sessions/:id — the meta overlay (title, pinned, archived, projects, model)
* `rename` — PATCH /api/sessions/:id — rename a session
* `delete` — DELETE /api/sessions/:id
* `convert` — POST /api/sessions/:id/convert — convert a session into another agent's store on the node
* `import` — POST /api/sessions/import — write session IR (or a history array) into an agent store
* `resume` — "Resume on…" — pull a session from the source node and write it into an agent store here
* `restore` — POST /api/sessions/:id/restore — revert workspace files via recorded diffs or a checkpoint ref
* `rewind` — POST /api/sessions/:id/rewind — truncate the transcript at a node, turn count or checkpoint

###### **Options:**

* `-h, --help` — Print help

## `peregrin sessions list`

GET /api/sessions — list the node's sessions

**Usage:** `peregrin sessions list [OPTIONS]`

###### **Options:**

* `--locks` — Also probe each agent for live lock state (?withLocks=1)
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin sessions create`

POST /api/sessions — create a session

**Usage:** `peregrin sessions create [OPTIONS]`

###### **Options:**

* `--cwd <CWD>` — Working directory the agent session runs in [default: /home/luis/GitHub/9aia/workspace/repos/peregrin]
* `--agent <AGENT>` — Agent runtime to spawn (default: the node's default agent)
* `--title <TITLE>` — Session title
* `--model <MODEL>` — Preferred model
* `--fallback <FALLBACK>` — Fallback model — repeatable, applied in order
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin sessions attach`

POST /api/sessions/:id/attach — attach live control (read-only while another process holds the lock)

**Usage:** `peregrin sessions attach [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--takeover` — SIGTERM the lock holder and attach writable
* `--model <MODEL>` — Preferred model for this attach
* `--fallback <FALLBACK>` — Fallback model — repeatable
* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin sessions prompt`

POST /api/sessions/:id/prompt — send one turn to an attached session

**Usage:** `peregrin sessions prompt [OPTIONS] <SESSION_ID> <TEXT>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node
* `<TEXT>` — The prompt text to send

###### **Options:**

* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions cancel`

POST /api/sessions/:id/cancel — stop the current run

**Usage:** `peregrin sessions cancel [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions permission`

POST /api/sessions/:id/permission — answer a pending permission request

**Usage:** `peregrin sessions permission [OPTIONS] --request <REQUEST> <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--request <REQUEST>` — The pending permission request id (requestId)
* `--option <OPTION>` — The option id to accept; omit to decline the request
* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions history`

GET /api/sessions/:id/history — the paginated backlog

**Usage:** `peregrin sessions history [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--limit <LIMIT>` — Number of trailing messages to return
* `--before <BEFORE>` — Exclusive end index — page backwards through the backlog
* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin sessions checkpoints`

GET /api/sessions/:id/checkpoints — workspace snapshot refs the store recorded

**Usage:** `peregrin sessions checkpoints [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin sessions export`

GET /api/sessions/:id/export — the complete session IR (feeds `sessions import`)

**Usage:** `peregrin sessions export [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--out <OUT>` — Write the session IR JSON to this file (default: stdout)
* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions stream`

GET /api/sessions/:id/stream — stream the live run's session events (sepia-proto SessionEvent JSON) to stdout

**Usage:** `peregrin sessions stream [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions meta`

PATCH /api/sessions/:id — the meta overlay (title, pinned, archived, projects, model)

**Usage:** `peregrin sessions meta [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--title <TITLE>` — Rename the session
* `--pin` — Pin the session
* `--unpin` — Unpin the session
* `--archive` — Archive the session
* `--unarchive` — Unarchive the session
* `--project <PROJECT>` — Project ids to assign — repeatable; replaces the whole list
* `--model <MODEL>` — Recorded model override
* `--clear-model` — Clear the recorded model override
* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions rename`

PATCH /api/sessions/:id — rename a session

**Usage:** `peregrin sessions rename [OPTIONS] <SESSION_ID> <TITLE>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node
* `<TITLE>` — The new title

###### **Options:**

* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions delete`

DELETE /api/sessions/:id

**Usage:** `peregrin sessions delete [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions convert`

POST /api/sessions/:id/convert — convert a session into another agent's store on the node

**Usage:** `peregrin sessions convert [OPTIONS] --to <TO> <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--to <TO>` — Agent store to convert into [possible values: cline, devin]
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin sessions import`

POST /api/sessions/import — write session IR (or a history array) into an agent store

**Usage:** `peregrin sessions import [OPTIONS] --to <TO> <PATH>`

###### **Arguments:**

* `<PATH>` — A session JSON export (`sessions export` / `peregrin export` output) or a history array

###### **Options:**

* `--to <TO>` — Agent store to write into [possible values: cline, devin]
* `--cwd <CWD>` — Override the imported working directory
* `--title <TITLE>` — Override the imported title
* `--model <MODEL>` — Override the imported model
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin sessions resume`

"Resume on…" — pull a session from the source node and write it into an agent store here

**Usage:** `peregrin sessions resume [OPTIONS] --to <TO> <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the source node

###### **Options:**

* `--to <TO>` — Agent store on the target node to resume into [possible values: cline, devin]
* `--source <SOURCE>` — Source node URL (default: same as --node) [default: http://127.0.0.1:18787]
* `--source-token <SOURCE_TOKEN>` — Bearer token for the source node (default: --token)
* `--from-agent <FROM_AGENT>` — Scope the source lookup to this agent's store
* `--cwd <CWD>` — Override the working directory on the target
* `--title <TITLE>` — Override the title on the target
* `--model <MODEL>` — Override the model on the target
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin sessions restore`

POST /api/sessions/:id/restore — revert workspace files via recorded diffs or a checkpoint ref

**Usage:** `peregrin sessions restore [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--confirm` — Required — restore writes real files under the session's cwd
* `--path <PATH>` — File to revert through the session's recorded diffs
* `--tool-call-id <TOOL_CALL_ID>` — Revert only this tool call's change to --path
* `--checkpoint <CHECKPOINT>` — Materialize a recorded snapshot ref (see `sessions checkpoints`)
* `--paths <PATHS>` — Narrow a --checkpoint restore to these files — repeatable
* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin sessions rewind`

POST /api/sessions/:id/rewind — truncate the transcript at a node, turn count or checkpoint

**Usage:** `peregrin sessions rewind [OPTIONS] <SESSION_ID>`

###### **Arguments:**

* `<SESSION_ID>` — Session id on the target node

###### **Options:**

* `--confirm` — Required — rewind deletes stored history
* `--node-id <NODE_ID>` — Keep this history node and everything before it
* `--turns <TURNS>` — Drop the last N user turns
* `--checkpoint <CHECKPOINT>` — Rewind to a recorded snapshot ref
* `--agent <AGENT>` — Scope the session id lookup to this agent's store — ids collide across agents
* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin campaign`

Campaign ops — load and run campaign yamls on this machine's campaign plane (run, validate, plus the submit+poll agent surface)

**Usage:** `peregrin campaign <COMMAND>`

###### **Commands:**

* `run` — Load a campaign yaml and execute it locally — prints the run's event log, one `ts source type` line per event
* `validate` — Expand a campaign yaml and summarize its wiring (includes, nodes, unemitting-subscription warnings)
* `submit` — Submit a workflow/bundle run on this machine's campaign plane — returns `{run_id, log}` immediately; poll `campaign status` / `campaign events` (the retired MCP `campaign_run`, submit+poll per P-028 — never blocks)
* `status` — Poll a `campaign submit` run — verdict, terminal event, counts
* `events` — Bounded tail of a `campaign submit` run's event log
* `serve` — Host N campaign yamls under one process — each gets its own supervisor (supersede queue, history, worker runs) while ALL `github-webhook` nodes share ONE listener that demuxes verified pushes by `repository.full_name` (all-match). SIGINT/ SIGTERM drains every campaign and exits

###### **Options:**

* `-h, --help` — Print help

## `peregrin campaign run`

Load a campaign yaml and execute it locally — prints the run's event log, one `ts source type` line per event

**Usage:** `peregrin campaign run [OPTIONS] <WORKFLOW>`

###### **Arguments:**

* `<WORKFLOW>` — Path to the campaign YAML

###### **Options:**

* `--dry-run` — Simulate the run without driver side effects
* `--replay <REPLAY>` — A JSON/JSONL event log to seed (replay) the run with
* `--history <HISTORY>` — Persist the run's event log to this JSONL file (ADR-0040's --history; `peregrin runs`/`explain`/`log` read it back)
* `--queue-store <QUEUE_STORE>` — Durable supersede-queue store (sqlite) — pending triggers and the in-flight run survive a restart and replay on boot. Only applies to `supersede:` campaigns with ingress nodes (supervisor mode)
* `-h, --help` — Print help

## `peregrin campaign validate`

Expand a campaign yaml and summarize its wiring (includes, nodes, unemitting-subscription warnings)

**Usage:** `peregrin campaign validate [OPTIONS] [PATH]`

###### **Arguments:**

* `[PATH]` — Path to the campaign YAML (or `--bundle`)

###### **Options:**

* `--bundle <BUNDLE>` — Installed bundle whose `main.yaml`/`main.yml` validates
* `--workflow <WORKFLOW>` — Workflow file inside `--bundle` (default `main.yaml`/`main.yml`)
* `-h, --help` — Print help

## `peregrin campaign submit`

Submit a workflow/bundle run on this machine's campaign plane — returns `{run_id, log}` immediately; poll `campaign status` / `campaign events` (the retired MCP `campaign_run`, submit+poll per P-028 — never blocks)

**Usage:** `peregrin campaign submit [OPTIONS] [PATH]`

###### **Arguments:**

* `[PATH]` — Path to the campaign YAML (or `--bundle`)

###### **Options:**

* `--bundle <BUNDLE>` — Installed bundle whose `main.yaml`/`main.yml` runs
* `--workflow <WORKFLOW>` — Workflow file inside `--bundle` (default `main.yaml`/`main.yml`)
* `--config <CONFIG>` — JSON object merged top-level into the campaign's `config:`
* `--secrets <SECRETS>` — JSON object of secret name→value for this run
* `--dry-run` — Simulate the run without driver side effects
* `--log <LOG>` — Explicit JSONL log path (default `<home>/runs/<run-id>.jsonl`)
* `-h, --help` — Print help

## `peregrin campaign status`

Poll a `campaign submit` run — verdict, terminal event, counts

**Usage:** `peregrin campaign status [OPTIONS] <RUN>`

###### **Arguments:**

* `<RUN>` — The run id `campaign submit` printed

###### **Options:**

* `--log <LOG>` — Read this log instead of `<home>/runs/<run>.jsonl`
* `-h, --help` — Print help

## `peregrin campaign events`

Bounded tail of a `campaign submit` run's event log

**Usage:** `peregrin campaign events [OPTIONS] <RUN>`

###### **Arguments:**

* `<RUN>` — The run id `campaign submit` printed

###### **Options:**

* `--type <EVENT_TYPE>` — Only events of this type
* `--limit <LIMIT>` — Max events to return (the tail) [default: 50]
* `--log <LOG>` — Read this log instead of `<home>/runs/<run>.jsonl`
* `-h, --help` — Print help

## `peregrin campaign serve`

Host N campaign yamls under one process — each gets its own supervisor (supersede queue, history, worker runs) while ALL `github-webhook` nodes share ONE listener that demuxes verified pushes by `repository.full_name` (all-match). SIGINT/ SIGTERM drains every campaign and exits

**Usage:** `peregrin campaign serve [OPTIONS] [PATHS]...`

###### **Arguments:**

* `[PATHS]...` — Campaign YAML files and/or directories of *.yaml/*.yml — every campaign found is hosted

###### **Options:**

* `--listen <LISTEN>` — Shared webhook listener bind address [default: 127.0.0.1:29173]
* `--path <PATH>` — Shared hook path (literal route) [default: /hook]
* `--history-dir <HISTORY_DIR>` — Per-campaign durable history dir — each campaign appends to &lt;dir&gt;/&lt;campaign-name&gt;.jsonl (`peregrin runs`/`log`/ `explain` read them back)
* `--queue-store-dir <QUEUE_STORE_DIR>` — Per-campaign durable supersede-queue dir — each campaign opens &lt;dir&gt;/&lt;campaign-name&gt;.db so pending triggers and the in-flight run replay per campaign on restart
* `-h, --help` — Print help

## `peregrin bundle`

Installed-bundle store ops (install/remove/list/info) — the tamias install pipeline under peregrin's store/cache roots

**Usage:** `peregrin bundle <COMMAND>`

###### **Commands:**

* `install` — Install a bundle ref (`./path`, `git:<url>[//subdir][#rev]`, `http(s) .tgz`) into the store
* `remove` — Drop an installed bundle's store entry (name or name@version) — the fetch cache is untouched
* `list` — List installed bundles — name@version, resolved sha, source ref
* `info` — A bundle's manifest, provenance and size

###### **Options:**

* `-h, --help` — Print help

## `peregrin bundle install`

Install a bundle ref (`./path`, `git:<url>[//subdir][#rev]`, `http(s) .tgz`) into the store

**Usage:** `peregrin bundle install [OPTIONS] <REFERENCE>`

###### **Arguments:**

* `<REFERENCE>` — The source ref to install

###### **Options:**

* `--allow-drivers` — Consent to register the package's bundled drivers
* `-h, --help` — Print help

## `peregrin bundle remove`

Drop an installed bundle's store entry (name or name@version) — the fetch cache is untouched

**Usage:** `peregrin bundle remove <NAME>`

###### **Arguments:**

* `<NAME>` — Store entry name (or `name@version`)

###### **Options:**

* `-h, --help` — Print help

## `peregrin bundle list`

List installed bundles — name@version, resolved sha, source ref

**Usage:** `peregrin bundle list`

###### **Options:**

* `-h, --help` — Print help

## `peregrin bundle info`

A bundle's manifest, provenance and size

**Usage:** `peregrin bundle info <NAME>`

###### **Arguments:**

* `<NAME>` — Store entry name (or `name@version`)

###### **Options:**

* `-h, --help` — Print help

## `peregrin logs`

JSONL run-log observability — scan dirs, tail events, explain runs (bounded; the live feed stays `events`)

**Usage:** `peregrin logs <COMMAND>`

###### **Commands:**

* `runs` — Scan a dir (recursive) for `*.jsonl` run logs, newest first — path, event count, verdict per run
* `events` — Tail events from a JSONL run log; `--type` filters to one event type, `--limit` bounds the tail (default 50)
* `explain` — Summarize one run log — verdict, terminal event, per-source/ per-kind counts, node failures, duration

###### **Options:**

* `-h, --help` — Print help

## `peregrin logs runs`

Scan a dir (recursive) for `*.jsonl` run logs, newest first — path, event count, verdict per run

**Usage:** `peregrin logs runs [OPTIONS]`

###### **Options:**

* `--dir <DIR>` — Dir to scan (default `<peregrin-home>/runs`)
* `--limit <LIMIT>` — Max runs to summarize [default: 20]
* `-h, --help` — Print help

## `peregrin logs events`

Tail events from a JSONL run log; `--type` filters to one event type, `--limit` bounds the tail (default 50)

**Usage:** `peregrin logs events [OPTIONS] <PATH>`

###### **Arguments:**

* `<PATH>` — Path to the JSONL run log

###### **Options:**

* `--type <EVENT_TYPE>` — Only events of this type
* `--limit <LIMIT>` — Max events to return (the tail) [default: 50]
* `-h, --help` — Print help

## `peregrin logs explain`

Summarize one run log — verdict, terminal event, per-source/ per-kind counts, node failures, duration

**Usage:** `peregrin logs explain <PATH>`

###### **Arguments:**

* `<PATH>` — Path to the JSONL run log

###### **Options:**

* `-h, --help` — Print help

## `peregrin projects`

Node-local project grouping (/api/projects)

**Usage:** `peregrin projects <COMMAND>`

###### **Commands:**

* `list` — GET /api/projects
* `create` — POST /api/projects — create a project
* `init` — Alias of `projects create` — init a project (repo) on the node
* `rename` — PATCH /api/projects/:id
* `delete` — DELETE /api/projects/:id

###### **Options:**

* `-h, --help` — Print help

## `peregrin projects list`

GET /api/projects

**Usage:** `peregrin projects list [OPTIONS]`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin projects create`

POST /api/projects — create a project

**Usage:** `peregrin projects create [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin projects init`

Alias of `projects create` — init a project (repo) on the node

**Usage:** `peregrin projects init [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin projects rename`

PATCH /api/projects/:id

**Usage:** `peregrin projects rename [OPTIONS] <PROJECT_ID> <NAME>`

###### **Arguments:**

* `<PROJECT_ID>`
* `<NAME>`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin projects delete`

DELETE /api/projects/:id

**Usage:** `peregrin projects delete [OPTIONS] <PROJECT_ID>`

###### **Arguments:**

* `<PROJECT_ID>`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin config`

Config ops — the node's server-side config (get/set)

**Usage:** `peregrin config <COMMAND>`

###### **Commands:**

* `get` — GET /api/config — the node's server-side UI state
* `set` — PATCH /api/config/:key

###### **Options:**

* `-h, --help` — Print help

## `peregrin config get`

GET /api/config — the node's server-side UI state

**Usage:** `peregrin config get [OPTIONS]`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `--json` — Print the raw API response as JSON
* `-h, --help` — Print help

## `peregrin config set`

PATCH /api/config/:key

**Usage:** `peregrin config set [OPTIONS] <KEY> <VALUE>`

###### **Arguments:**

* `<KEY>`
* `<VALUE>` — JSON when parseable, else a plain string

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin push`

Web-push subscription management (/api/push/*)

**Usage:** `peregrin push <COMMAND>`

###### **Commands:**

* `vapid` — GET /api/push/vapid — the node's web-push public key
* `subscribe` — POST /api/push/subscribe
* `unsubscribe` — DELETE /api/push/subscribe

###### **Options:**

* `-h, --help` — Print help

## `peregrin push vapid`

GET /api/push/vapid — the node's web-push public key

**Usage:** `peregrin push vapid [OPTIONS]`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin push subscribe`

POST /api/push/subscribe

**Usage:** `peregrin push subscribe [OPTIONS] <SUBSCRIPTION>`

###### **Arguments:**

* `<SUBSCRIPTION>` — A push subscription JSON ({endpoint, keys:{auth,p256dh}, prefs?})

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin push unsubscribe`

DELETE /api/push/subscribe

**Usage:** `peregrin push unsubscribe [OPTIONS] <ENDPOINT>`

###### **Arguments:**

* `<ENDPOINT>`

###### **Options:**

* `--node <NODE>` — Base URL of the peregrin node (default: PEREGRIN_NODE_URL, else http://127.0.0.1:18787) [default: http://127.0.0.1:18787]
* `--token <TOKEN>` — Bearer token for the node (default: PEREGRIN_TOKEN)
* `-h, --help` — Print help

## `peregrin pair`

Print a one-time pairing code — the UI exchanges it via POST /api/pair

**Usage:** `peregrin pair [OPTIONS]`

###### **Options:**

* `--home <HOME>` — PEREGRIN_HOME of the node to pair with [default: /home/luis/.local/share/peregrin]
* `--url <URL>` — The node's URL, printed for the pairing UI [default: http://localhost:18787]
* `-h, --help` — Print help

## `peregrin serve`

Serve the peregrin node — the `/api/*` surface (PEREGRIN_* env configures it)

**Usage:** `peregrin serve [OPTIONS]`

###### **Options:**

* `--ephemeral` — Ephemeral/CI node mode — SIGTERM-aware shutdown plus a snapshot flush of live/locked entities on exit (`PEREGRIN_EPHEMERAL=1` is the env equivalent)
* `-h, --help` — Print help

## `peregrin import`

Foreign config → peregrin node-config workflow conversion (ADR-0081 bupha importer)

**Usage:** `peregrin import <COMMAND>`

###### **Commands:**

* `bupha` — Convert a bupha.toml/bupha.yaml into a node-config workflow (ingress github-webhook node + per-repo/job gate pipelines)

###### **Options:**

* `-h, --help` — Print help

## `peregrin import bupha`

Convert a bupha.toml/bupha.yaml into a node-config workflow (ingress github-webhook node + per-repo/job gate pipelines)

**Usage:** `peregrin import bupha [OPTIONS] <PATH>`

###### **Arguments:**

* `<PATH>` — Path to the bupha config (TOML or YAML — extension picks the parser)

###### **Options:**

* `--out <OUT>` — Write the emitted workflow to this file (default: stdout)
* `-h, --help` — Print help

## `peregrin runs`

Recent runs — per-push records read back from the durable JSONL history (newest first)

**Usage:** `peregrin runs [OPTIONS]`

###### **Options:**

* `--history <HISTORY>` — The event log to read (default: $PEREGRIN_HOME/history.jsonl)
* `--repo <REPO>` — Filter to one repo ("owner/name" also matches its job keys)
* `-n, --limit <LIMIT>` — Limit to the newest N runs
* `-h, --help` — Print help

## `peregrin log`

Raw event log — the last N lines of the JSONL history

**Usage:** `peregrin log [OPTIONS]`

###### **Options:**

* `-n, --n <N>` — Lines to show [default: 50]
* `--history <HISTORY>` — The event log to read (default: $PEREGRIN_HOME/history.jsonl)
* `-h, --help` — Print help

## `peregrin explain`

Why a sha did/didn't run — the run's events + outcome, or the leftover worktree it died in

**Usage:** `peregrin explain [OPTIONS] <TARGET>`

###### **Arguments:**

* `<TARGET>` — The pushed sha (any prefix) or an event id

###### **Options:**

* `--history <HISTORY>` — The event log to read (default: $PEREGRIN_HOME/history.jsonl)
* `--config <CONFIG>` — Node config to read the worktree base from
* `-h, --help` — Print help

## `peregrin version`

Print the peregrin version stamp

**Usage:** `peregrin version`

###### **Options:**

* `-h, --help` — Print help
