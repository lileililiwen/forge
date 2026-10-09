# Tasks: GitHub gh-fallback and create register-if-missing

## Phase 1 — BFS baseline

- [x] 1.1 Confirm no active change (`openspec list`), names preflight PASS, record Gap1/Gap2 against current CLI output (`github-adapter-unavailable`, 7-row `forge list`).
- [x] 1.2 Map adapter (`src/github/adapter/model.rs`), cli (`src/github/cli/`), CLI handlers (`src/cli/project.rs`, `src/cli/github.rs`, `src/cli/commands_ops.rs`), portal (`src/portal/sections.rs`), web (`frontend/`) touch points; confirm `gh repo view/edit` flags.
- [x] 1.3 Set `current_spec: github-gh-fallback-register` in HANDOFF.

## Phase 2 — DFS requirement implementation

- [x] 2.1 `src/github/gh_fallback.rs`: observe builder/runner (`gh repo view --json`), direct topic propose runner (`gh repo edit --add-topic`), topic-only gate, redaction, failure classification.
- [x] 2.2 `src/cli/project.rs`: adapter-first observe/propose with `gh` fallback branches + `adapter_source=gh-cli-fallback` envelope.
- [x] 2.3 `src/cli/commands_ops.rs` + `src/cli/github.rs`: `create --register-if-missing` (manifest validate → register → `gh repo create`), `registered` envelope.
- [x] 2.4 Portal `repositories` controls for observe/propose/create.
- [x] 2.5 Web `GitHub metadata` card (topics list + dev-highlight + text alternative, propose form with confirm field, create flow with visibility radios + push-source checkbox) + appended CSS + `initGithubMetadata()` wiring.

## Phase 3 — BFS regression/completeness

- [x] 3.1 New `tests/github_gh_fallback_register_contract.rs` green (builders, gates, redaction, register-if-missing, frontend/portal tokens).
- [x] 3.2 Full `cargo test` shows no new failure vs baseline (pre-existing failures recorded, none attributable).
- [x] 3.3 Adapter-configured path byte-identical; PR-mode propose still requires the adapter; create without the flag unchanged; no new API route; tokens absent from all output.

## Phase 4 — Verification

- [x] 4.1 `cargo fmt --check` / `cargo build` clean (pre-existing warnings only).
- [x] 4.2 `cargo test` (workspace) executed; contract test green.
- [x] 4.3 `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` PASS; `git diff --check` clean.
- [x] 4.4 `forge gate --dry-run` rehearsed + bounded full `forge gate` run; verdict recorded in HANDOFF evidence.
