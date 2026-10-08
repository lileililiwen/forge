# Proposal: Automatic workspace sync command

## Why

Onboarding workspace siblings currently requires one CLI invocation per
directory (`forge import --accept` / `forge register`) or one browser batch
per 25 directories. For an expanding workspace of 70+ siblings that is
manual toil, not a mechanism: it cannot be re-run, scheduled, or delegated,
and every new sibling restarts the work. A single repeatable command must
bring a whole workspace root into the registry and report exactly what it
did.

## What Changes

- Add `forge workspace sync [ROOT]`: scan the immediate children of ROOT
  (default: current directory), and for each one either register its
  manifest, adopt it via import detection, keep its existing registration,
  or skip it with a machine-readable reason. One run, one report, exit code
  reflecting failures only.
- Drive it through a new Core function `import::sync_workspace` reusing
  `inspect_import`, `adopt_import`, `derive_project_id` and
  `Registry::register` unchanged; the CLI only renders the report
  (human table plus `--format json`).
- Skips are explicit, never silent: ambiguous detection, undecidable stack,
  invalid manifest, hidden entries, non-directories, symlinks and identity
  collisions each get a named reason. Already-registered directories report
  `already` without rewriting.
- Running sync again after adding siblings onboards only the new ones;
  re-running over a synced workspace reports everything `already`.

## Package Boundary and Split Assessment

One independently verifiable outcome: a repeatable CLI command that
converges a workspace root into the registry with a complete per-directory
report. Scanning, deciding, adopting and reporting share one Core function,
one journal contract and one test oracle; splitting them would ship
unusable halves. The browser onboarding flow (shipped) and the dashboard
rework (queued as `forge-web-human-dashboard`) are separate surfaces with
separate oracles and stay untouched.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-workspace-sync` (**this**) | `forge workspace sync` converges a root into the registry with a per-directory report | Forge `src/import` + `src/main.rs` / Rust | `import::sync_workspace`; `WorkspaceCommands::Sync` | `project-import` (Core adoption semantics) | `tests/workspace_sync_contract.rs` incl. rerun-idempotence |
| `forge-web-human-dashboard` (queued) | Human-centered dashboard language and layout | Forge `frontend/` | DOM/copy only | this package (fleet must be full first) | browser drive |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner / release boundary | Decision |
|---|---|---|---|---|---|
| Import adoption | `src/import/mod.rs` `inspect_import` / `adopt_import` / `derive_project_id` | Detection, manifest writing, identity derivation | Per-directory; needs a bounded driver | `src/import` owns adoption | **extend shared owner** — add a directory-walking driver, change no semantics |
| Registration | `src/registry/mod.rs` `register` / `check_identity_available` | Manifest validation, collision rules | Per-directory; unchanged | `src/registry` owns persistence | **reuse unchanged** |
| CLI command tree | `src/main.rs` `Import` / `Graduation` subcommand pattern | Clap derivation, `--format` rendering | No bulk verb exists | `src/main.rs` owns dispatch | **extend shared owner** — new `Workspace` top-level command |
| Web bulk onboard | `src/api/workspace.rs` `workspace_onboard_write` | Preview/confirm/digest discipline | Browser-only; CLI needs no confirmation gate (the operator typed the command) | `src/api` owns web routes | **keep local** — CLI reuses Core directly, no route involved |

No sibling checkout is touched; no host folder is assumed (ROOT is a CLI
argument defaulting to current directory, exactly like `import`/`register`).

## User Experience and Interface Impact

Actor: the operator in a terminal managing an expanding workspace. Command:
`forge workspace sync /path/to/workspace` (or `.` by default). Output: one
line per directory (`ok <id>`, `already <id>`, `skipped <reason>`,
`failed <reason>`) plus a summary count; exit 0 unless a failure occurred.
JSON shape mirrors the human report for scripting. `UI/UX: N/A` for browser
work — this is a CLI mechanism; the browser journey it unblocks is owned by
`forge-web-human-dashboard` and the shipped onboarding panel.

## BFS Impact Map

- **Capabilities:** `forge-workspace-sync` (new: CLI command + Core report).
- **Users / flows:** one terminal invocation converges a whole root;
  re-runs converge incrementally.
- **Contracts / data / persistence:** no schema change. Per-directory
  `register` rows as today; one `workspace.sync` summary row with counts.
- **Integrations / configuration:** ROOT is a CLI argument (default `.`);
  no environment key, no assumed folder.
- **Callers:** `src/main.rs` dispatch + `import::sync_workspace`; catalog
  gains one honest row (new CLI command must be catalogued — the catalog
  parity test enforces it).
- **Failure / boundary behavior:** ambiguous → skipped with reason (never
  guessed); undecidable → skipped; invalid manifest → failed; collisions →
  failed with typed reason; symlinks/hidden → skipped; empty root → ok
  with zero counts.
- **Tests:** new `tests/workspace_sync_contract.rs` (fixture root,
  mixed shapes, rerun-idempotence, JSON shape, exit codes).
- **Privacy / security:** CLI may read operator-named paths by design;
  error text scrubs nothing new (Core messages already path-aware).

## Capabilities

- `forge-workspace-sync`: automatic convergence of a workspace root into
  the registry.

## Non-goals

- No browser surface (queued separately).
- No recursive descent below immediate children.
- No guessing for ambiguous directories (explicit `--profile` remains
  per-directory, or fix in UI later).
- No removal/deletion of registry rows for vanished directories (report
  only; pruning is a separate destructive decision).
- No change to single import/register semantics, journal schema or
  `API_CONTRACT_VERSION`.
