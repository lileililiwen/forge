# Design: Retire migrated origin repositories

## Ownership and target boundary

Forge owns imported source and its continued development. GitHub owns the
remote archived flag. The action uses authenticated `gh` calls as the named
operator and targets only this explicit migrated-origin allowlist:

| Origin | Forge-owned source |
|---|---|
| `lileililiwen/driftwatchdog` | `crates/driftwatchdog/` |
| `lileililiwen/cargo-audit-boost` | `crates/cargo-audit-boost/` |
| `lileililiwen/cargo-matrix-boost` | `crates/cargo-matrix-boost/` |
| `lileililiwen/cargo-sonar-boost` | `crates/cargo-sonar/` |
| `lileililiwen/workspace-governance` | `modules/workspace-governance/` |
| `lileililiwen/dotnet-platform-libs` | `modules/dotnet-platform-libs/` |
| `lileililiwen/rust-platform-libs` | `modules/rust-platform-libs/` |

This is an allowlist, not a name-discovery rule. A repository outside it is
never eligible. The three archived modules use updated Forge-owned repository
metadata where applicable; legacy references in history or vendor metadata do
not enlarge the target set.

## Preconditions and evidence

Before any remote write:

1. Require Forge `main` and a clean Forge worktree with this change's
   implementation/archive commit present.
2. Run all available source import verifiers for Workspace Governance, .NET
   and Rust platform libraries. Verify their fixed source revisions and
   source-only commits are ancestors in Forge history.
3. Verify imported Rust workspace crates are root workspace members and
   module Gate declarations, CI workflows and Governance registrations point
   into Forge. Search for active sibling checkout paths in maintained source
   and workflows; historical docs and archived provenance records are
   classified separately.
4. Capture local Git status for the original Workspace Governance, .NET and
   Rust checkouts. Do not write to any of them; preserve existing untracked
   files exactly.
5. Query the GitHub API for every allowlisted repository and classify each as
   `archived`, `active`, `absent`, or `unavailable`. A `404` is `absent` only
   when the authenticated API response is authoritative for the account; auth,
   network, rate-limit and permission errors are `unavailable`, never absence.

The audit baseline observed during proposal authoring is not authority for the
later operation. Re-query the target directly before every archive call.

## Remote operation and recovery

For each live target immediately before action:

- If `archived=true`, record `already_archived`; do not write.
- If the API confirms the exact repository is absent, record `absent`; do not
  create or substitute a target.
- If the state cannot be read, stop the remote phase and record the exact
  error. Do not infer or force a state.
- If active, archive only that exact allowlisted GitHub repository, then query
  it again and require `archived=true`. A failed confirmation remains
  `unresolved`; do not report completion.

Archive each repository separately so partial completion is explicit. The
GitHub archive flag can be reversed by an authorized owner if needed, but no
automatic unarchive or rollback is attempted. Do not delete local checkouts or
rewrite their worktrees. No Forge push or package publication occurs.

## Consumer verification

The migrated source remains runnable inside Forge: Cargo members stay under
`crates/*`; module Gate manifests remain under `modules/*`; existing source
import checkers continue to use Forge's committed source history; module CI
and owner-gated publishers remain rooted in Forge. Run the import checkers,
the documented workspace checks, the full Forge Gate dry-run and bounded full
Gate after the remote changes. If a check uses an origin checkout or remote
source, identify and migrate that consumer before archiving; otherwise it
blocks the archive operation.

## Evidence and completion

HANDOFF records the exact observation timestamp and per-repository state:
archived by this change, already archived, confirmed absent, or unresolved.
It also records provenance/consumer checks, unchanged pre-existing checkout
statuses, OpenSpec archive and canonical promotion, local Gate verdicts and
both commit identifiers. Any unavailable API result or active undeclared
consumer blocks completion. Nothing is pushed.
