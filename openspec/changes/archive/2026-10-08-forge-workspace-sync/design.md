# Design: Automatic workspace sync command

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate
  (library + binary). No sibling touched.
- **Modules changed:**
  - `src/import/mod.rs` — add `WorkspaceSyncEntry`, `WorkspaceSyncReport`,
    `pub fn sync_workspace(registry, root)`.
  - `src/main.rs` — add `Workspace` top-level command + `WorkspaceCommands`
    enum with `Sync { root, format }`; dispatch `cmd_workspace_sync`;
    render human table + `--format json`.
  - `src/api/command_catalog.rs` — catalogue the new command row(s);
    update the pinned 227 count and web/execution pin lists (new rows are
    CLI-only/read honest states; count becomes 228).
  - `tests/workspace_sync_contract.rs` — new oracle.
- **Modules reused unchanged:** `inspect_import`, `adopt_import`,
  `derive_project_id`, `Registry::register`, `check_identity_available`,
  manifest loading.
- **Must NOT change:** single import/register behavior, bearer routes, web
  routes, journal schema, `API_CONTRACT_VERSION`, fleet aggregation.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. Build: `cargo build`. Test:
  `cargo test --test workspace_sync_contract` plus catalog parity
  (`--bin forge`), `catalog_contract`, management contract.
- Linux; reads operator-named directories only.

## 3. Ownership and shared code

- `src/import` owns the driver: `sync_workspace(&mut Registry, &Path)`
  walks sorted immediate children, skips hidden/non-dir/symlink entries,
  and per directory returns one closed outcome. Detection/adoption stay
  the single Core implementations.
- `src/main.rs` owns CLI parsing/rendering only.

## 4. User experience and interface

Terminal UX: `forge workspace sync [ROOT]` (default `.`). Stdout: one
`ok|already|skipped|failed <leaf> [<detail>]` line per directory in sorted
order, then `synced N, already M, skipped K, failed F`. Exit 0 iff F == 0
(skips do not fail the run). `--format json` emits the same report as a
`forge-workspace-sync/0.1.0` envelope. No prompts, no interaction.

## 5. Behavioral model

Per directory (sorted by leaf):
1. Skip (reason `hidden`/`not-a-directory`/`symlink`) — no read.
2. Canonicalize; escape of ROOT → `failed: outside-root` (defensive; a
   validated join cannot produce this, but symlinks are re-checked).
3. `forge.yaml` present → `register` → `ok` (or `failed` with typed reason,
   e.g. unknown profile, id collision).
4. Else `inspect_import(None)`:
   - decidable → `adopt_import(None, None)` → `ok`;
   - ambiguous → `skipped: ambiguous (re-run one directory with --profile)`;
   - undecidable → `skipped: undecidable`.
5. Already registered at the same canonical path → `already` (register
   refresh still runs? No — report `already` WITHOUT rewriting: check via
   `check_identity_available`-style read first... simpler: attempt register;
   same id+path succeeds as refresh and we report `already` when the id was
   already present at that path before the call. Implement by snapshotting
   `registry` project list up front: if id present at same path →
   `already`, skip Core call. If id present at different path → `failed:
   id-collision` without touching anything.)
6. Append one `workspace.sync` journal row (`done` iff F == 0 else
   `failed`) with counts only.

Determinism: sorted leaves, fixed vocabulary, no timestamps in stdout
(except journal, as today).

## 6. Contract and compatibility

- JSON envelope: `{contract: "forge-workspace-sync/0.1.0", root, entries:
  [{directory, outcome, id?, profile?, reason?}], summary: {ok, already,
  skipped, failed}}`. Echoing ROOT is safe here (the operator typed it;
  browser no-echo rules do not apply to the terminal).
- Catalog: new `workspace` group + `sync` row, CLI-only with reason
  (bulk terminal operation, no browser route). Count 227 → 228; update
  pinned tests.
- CLI dispatch and existing outputs unchanged.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| ROOT missing/not-a-dir | typed error, exit non-zero, nothing touched |
| Empty root | ok, all counts zero |
| Ambiguous dir | skipped + reason, continues |
| Undecidable dir | skipped + reason, continues |
| Invalid manifest | failed + typed reason, continues |
| Id/path collision | failed + typed reason, touches nothing |
| Provider/config missing | N/A (no provider involved; local profiles only) |
| Any failure | exit non-zero after processing all dirs |

## 8. Verification oracle

- **New `tests/workspace_sync_contract.rs`** on a fixture root (manifest,
  importable, ambiguous, undecidable, invalid-manifest, hidden, file,
  symlink-escape, pre-registered): asserts per-entry outcomes, summary
  counts, exit codes, JSON envelope shape; rerun reports all `already`
  with zero writes beyond the summary row; ambiguous rerun with
  `--profile`? (out of scope — single import covers it).
- **Catalog parity:** `--bin forge` integrity/coverage tests green at 228.
- **Existing suites green:** import/register unit tests, management
  contract, `catalog_contract`.
- A task box is checked only with the command output for its assertion.

## 9. Decision ledger

- **Resolved:** new top-level `workspace` command (not an `import --all`
  flag) so future workspace verbs have a home and `--help` stays readable.
- **Resolved:** skips never fail the run; failures do. Reruns are
  idempotent by identity check, not by blind re-adoption.
- **Resolved:** no pruning of vanished directories — destructive, separate
  decision.
- **Resolved:** ROOT echoed in CLI output is safe (operator-supplied);
  browser rules do not apply to the terminal.
- **Blockers:** none.
