# Tasks: Operator login credentials on `scripts/web.sh start`

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta specs agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/identity/global.rs` (`setup`, `change_password`, `generate_password`, `is_configured`, `authenticate`, `session_valid`), `src/main.rs` (`IdentityCommands` at ~1315, `cmd_identity` at ~9410, `read_secret` at ~9893, `db_path` at ~2652), `src/api/command_catalog.rs` (identity rows at ~2257, pinned count test at ~2632) and `scripts/web.sh`.
- [x] Verify `forge identity *` and `forge api serve` resolve the same `default_registry_path` when no `--registry` is passed (`src/main.rs` builds one `db_path` for both); record the evidence.
- [x] Map each requirement and scenario to its Core function, CLI shape, catalog row or script function and contract-test assertion.
- [x] Add `tests/web_login_credentials_contract.rs` with the design §8 scenarios.

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/identity/global.rs`: add `pub fn email(db_path) -> Result<Option<String>, String>` reading only the `email` column.
- [x] `src/main.rs`: add `IdentityCommands::Status` and its human/JSON rendering (`configured:`/`email:`; `forge-admin-login/1.0.0` JSON).
- [x] `src/main.rs`: add `#[arg(long)] password_stdin: bool` to `Setup` and `ChangePassword`; add `read_password_stdin`; keep the interactive path unchanged; dispatch the new fields.
- [x] `src/api/command_catalog.rs`: add the `identity.status` CLI-only row; update the pinned count 233 → 234 with a delta comment.
- [x] `scripts/web.sh`: add `--admin-email`, `ensure_admin`, `reset-password`, banner printing, and the documented `FORGE_WEB_RUN_DIR` / `FORGE_WEB_LOG_DIR` / `FORGE_WEB_ROOT_DIR` / `FORGE_BIN` seams; update `usage`.
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify the interactive `identity setup` / `change-password` behavior, messages and exit codes are unchanged.
- [x] Verify no other CLI path, web route, registry/journal schema or `API_CONTRACT_VERSION` changed.
- [x] Verify the catalog parity/integrity tests pass and the new row is honest (`cli_only`, no route).
- [x] Verify the password never reaches `argv`, `.forge/run/state`, or any log; `identity status` never reads the hash.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test --test web_login_credentials_contract` green (new).
- [x] `cargo test --lib command_catalog`, `cargo test --bin forge`, `--test plugins_contract`, `--test catalog_contract`, `--test classify_derive_contract`, `--test classify_apply_contract`, `--test forge_web_command_catalog_contract`, `--test forge_web_maintainer_surface_contract` green. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] `forge gate --dry-run`, then `forge gate --timeout-secs 600` with the verdict recorded in HANDOFF; no new failure attributable to this change.
- [x] Archive with `openspec archive web-login-credential-bootstrap --yes` (no `--skip-specs`); promote the canonical specs; update HANDOFF with the evidence and pointer handling.
