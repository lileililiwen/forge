# Design: Reuse the user's authenticated GitHub CLI

## 1. Implementation boundary

Repository `/home/paul/code/forge`, Rust 2021/MSRV 1.87. New module
`src/github/cli.rs` owns the versioned request/response contract
(`forge-github-cli-workflows/0.1.0`), the bounded argument-array
invocation of the installed `gh` CLI, the closed [`GhOutcome`] state
mapping, and the journaled audit row. The new subcommands hook into
`src/main.rs` under `Commands::Project { ProjectCommands::Github {
GithubCommands::Auth|Clone|Create|PullRequest } }`. Reuse
`crate::policy::redact_credentials`, the `spawn_with_timeout` helper
in `src/github/adapter.rs` (extract it into `src/process.rs` so both
adapters share one bounded spawn path), the existing `ForgeError`
taxonomy, and the existing `Registry::record_operation` journal
entry. **Do not** modify `src/github/adapter.rs`,
`src/github/normalize.rs`, `src/catalog/source.rs`, or the
`forge-github-metadata/0.1.0` JSON contract — the metadata surface
remains read-only and token-based; the new CLI surface is a
separate, optional, owner-authenticated workflow boundary.

## 2. Language and runtime

Rust with `std::process::Command` and the installed `gh` CLI. Every
child process uses an executable path plus a bounded `OsString`
argument array, a 30-second per-call wall-clock timeout (clamped to
`1..=120`), a 2 KiB stderr cap (truncated with a `[truncated]`
marker), and no shell. The binary is resolved in this order:

1. `$FORGE_GH_BIN` if set and the file is an executable regular
   file.
2. `gh` on `$PATH`.

The user's `gh` auth context is read through `gh auth status`; Forge
never spawns `gh auth token`, never reads the credential store
directly, and never inherits the credential through any environment
variable of its own. Verification uses a fake `gh` shell-script
binary planted in a temporary directory and exposed through a
controlled `PATH`; live validation is limited to `gh auth status`
and a read-only `gh repo view` when the user has configured an
account.

## 3. Ownership and shared code

Forge owns the project workflow (`src/github/cli.rs` and the CLI
subcommand). GitHub CLI owns login state, the credential store,
hosts and scopes. Forge does **not** introduce a common auth
service, token store, or token redaction layer of its own — the
existing `policy::redact_credentials` redactor is the single
credential-shaped filter shared across every Forge surface. No
Workspace Governance or platform-library change is required; the
existing `github-project-metadata-adapter` and `project-catalog-query-contract`
stay byte-compatible.

## 4. Behavioral model

The new contract is `forge-github-cli-workflows/0.1.0`. The request
envelope is:

```json
{
  "operation": "auth" | "clone" | "create" | "pull-request",
  "project_id": "<kebab-case-or-null>",
  "repository": "<owner/repo-or-null>",
  "destination": "<absolute-path-or-null>",
  "visibility": "private" | "public" | null,
  "title": "<string-or-null>",
  "body": "<string-or-null>",
  "operation_id": "<stable-id>"
}
```

The response envelope is:

```json
{
  "contract": "forge-github-cli-workflows/0.1.0",
  "operation_id": "<stable-id>",
  "outcome": "done" | "auth-required" | "forbidden" | "not-found"
           | "conflict" | "rate-limited" | "timeout"
           | "unavailable" | "failed",
  "exit_code": <number-or-null>,
  "artifact_url": "<string-or-null>",
  "stderr_tail": "<string-or-null>",
  "note": "<string>"
}
```

Subcommands and their fixed `gh` argument vectors:

- `forge project github auth [--hostname <host>]` →
  `gh auth status --hostname <host>`. Read-only; no journal row.
- `forge project github clone <owner/repo> <destination> [--confirm]` →
  `gh repo clone <owner/repo> <destination>`. The destination must
  be absent or empty; an existing destination is `conflict`.
  `--confirm` is required so an unexpected clone is not implicit;
  the journal records one `github.cli` row with the captured
  destination path.
- `forge project github create <project> --repo <owner/name>
  [--visibility private|public] [--confirm-public]
  [--push-source --confirm]` → `gh repo create <name> --source
  <path> --remote origin --private` for the private default, or
  `gh repo create <name> --source <path> --remote origin --public`
  when both `--visibility public` and `--confirm-public` are
  passed. `--push-source --confirm` adds `--push` so the local
  commits are published; without `--confirm` the push is refused
  even when `--push-source` is set.
- `forge project github pull-request <project> --title <title>
  --body <body> [--draft] --confirm` → `gh pr create --title <title>
  --body <body> [--draft]`. The handler runs `git status` and `git
  remote get-url origin` first; a dirty tree, missing remote, or
  remote pointing outside `github.com` is `conflict`. Without
  `--confirm` the call is refused before `gh` is spawned.

## 5. Contract and compatibility

The closed state vocabulary maps to the typed errors:

| Outcome | CLI/MCP/API code | Source |
| --- | --- | --- |
| `done` | exit 0 | `gh` returned 0 with an artifact URL or empty stdout |
| `auth-required` | `github-cli-auth-required` | `gh auth status` reports no logged-in user |
| `forbidden` | `github-cli-invalid` | `gh` reports scope or permission failure |
| `not-found` | `github-cli-invalid` | repository identity rejected by `gh` |
| `conflict` | `github-cli-conflict` | destination exists, dirty tree, missing remote |
| `rate-limited` | `github-cli-unavailable` | `gh` reports a rate-limit hit |
| `timeout` | `github-cli-unavailable` | child killed after 30 s wall clock |
| `unavailable` | `github-cli-unavailable` | binary missing, non-executable, or PATH empty |
| `failed` | `github-cli-invalid` | `gh` returned nonzero without mapping |

The existing `forge-github-metadata/0.1.0` JSON contract is
**unchanged**. The metadata adapter remains the read-only
observation surface for token-based automation; the new CLI
surface is a separate, owner-authenticated workflow boundary. The
two share only the `policy::redact_credentials` redactor and the
shared `spawn_with_timeout` helper.

## 6. Failure and boundary policy

- **Missing `gh`** → typed `github-cli-unavailable`, no process
  spawned.
- **No authenticated account** → typed
  `github-cli-auth-required` with the recovery command
  `gh auth login --hostname <host>`. Forge does not execute it.
- **Unknown scope or repository** → `forbidden` / `not-found` with
  the stderr tail (≤ 2 KiB, credential-redacted).
- **Dirty tree, missing remote, or existing origin** → refuse
  before remote write with `github-cli-conflict`.
- **Public visibility without `--confirm-public`** → refuse before
  `gh` is spawned with `github-cli-invalid`.
- **`--push-source` without `--confirm`** → refuse with
  `github-cli-invalid`.
- **Timeout** → terminate the child, mark outcome `timeout`, and
  surface a hint that the remote may have accepted the write so the
  next call first queries the remote rather than repeating blindly.
- **Unknown operation** → reject before process launch.
- **Credentials**: no log line, no error, no journal row may contain
  a credential-shaped substring. The redactor is the single
  `policy::redact_credentials` filter every Forge surface already
  uses.

## 7. Verification oracle

The verification oracle is a fake `gh` shell-script binary planted
in a temporary directory and exposed through a controlled `PATH`
(plus the inherited system `PATH` so `git` and other tools remain
reachable). The fake binary asserts exact argv for every
operation, simulates `gh auth status`, `gh repo clone`,
`gh repo create`, and `gh pr create`, and writes a structured log
file so tests can confirm the exact argument array Forge produced.
Tests cover:

- Private default + missing `--confirm-public` for public.
- Public with `--confirm-public`.
- `--push-source` with and without `--confirm`.
- Existing destination / dirty tree / missing remote /
  credential-shaped topic.
- Token never echoed on stdout / stderr / journal row.
- Existing `forge-github-metadata/0.1.0` adapter remains
  byte-compatible.

CLI integration tests assert help, typed exit codes, and the
JSON envelope for every subcommand. Optional real smoke captures
`gh auth status` and a read-only `gh repo view`; it must not create
or mutate a remote repository.

## 8. Decision ledger

Resolved: reuse `gh` auth without exposing tokens; GitHub writes are
explicit and require confirmation; local Git remains `git`; private
repository creation is default; existing token-based metadata
adapter stays compatible; shared spawn-with-timeout helper is
extracted to `src/process.rs` so both adapters own one bounded
spawn path.

Deferred: enterprise hosts (only `github.com` is wired today),
GitHub App installation tokens, remote Forge workers, repository
deletion, release creation, PR merge, and `--web` browser flow.
These follow the same pattern when added and require no new
contract.

Blockers: none. The fake `gh` binary is the production oracle; live
verification stays read-only.