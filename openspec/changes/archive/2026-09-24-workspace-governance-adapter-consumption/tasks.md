# Tasks: Consume the Workspace Governance adapter end to end

## 1. BFS — Baseline and impact coverage

- [x] Map provider resolution, `providers.yaml` persistence, list/status/
  use/inspect callers and the observation normalization entry points.
- [x] Capture real `workspace_check.py --audit --json` output shapes (clean,
  ERROR, adoption gap, unregistered) from the sibling repo as fixture seeds.
- [x] Define the refusal message texts for unresolved/preset-failing paths.
- [x] Add fixture stubs exercising all four mappings before implementing.

## 2. DFS — Requirement-by-requirement implementation

- [x] Implement preset resolution (flag > env > candidate file checks) with
  `--workspace-root` on `governance use`.
- [x] Persist the resolved adapter path in provider selection without
  touching manifest or registry identity.
- [x] Wire the four audit-shaped fixtures through contract tests, asserting
  status mapping, evidence bounding and redaction.
- [x] Update docs/provider-evidence.md with the `workspace-governance` row
  and README optional-provider section.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-verify the standalone default: no config, no env, no sibling →
  local provider passes and nothing requests a workspace root.
- [x] Re-verify explicit `--adapter` precedence and provider switching
  byte-stability across CLI/MCP/API/portal read surfaces.
- [x] Re-verify unavailable/incompatible classification with a broken preset
  adapter cannot render healthy anywhere in doctor or portal.
- [x] Confirm no filesystem scanning or network calls were introduced.

## 4. Verification

- [x] `cargo fmt --all -- --check`; `cargo build`; `cargo test --all-targets`
  (recorded exclusions only as before).
- [x] `cargo clippy --all-targets -- -D warnings`.
- [x] `node scripts/check-openspec-change-names.mjs`; `openspec validate
  --all --strict --no-interactive`; `git diff --check`.
- [x] If the sibling checkout with its adapter exists on the host: run one
  live `forge governance status .` round trip and record evidence or the
  exact failed command as the next action.

## Evidence notes

- Live captures (2026-09-24, sibling HEAD `204d140`) and the design
  corrections they forced (packaged candidate nests under
  `<workspace-root>/workspace-governance/scripts/`; the explicit root is
  re-supplied to the adapter as `WORKSPACE_ROOT`; the real sibling file is
  git mode `100644` so the preset honestly refuses it naming the exact
  path — next action: sibling `git update-index --chmod=+x`) are recorded in
  `tests/fixtures/governance-audit/NOTES.md`,
  `docs/provider-evidence.md` and `HANDOFF.md`.
- The full audit-shaped matrix (`pass`/`fail`/`blocked`/`unknown`) ran live
  end to end through `governance use --workspace-root` + `governance status`
  against a byte-identical scratch copy of the real adapter code; removal of
  the configured checkout yields `unavailable` while local surfaces continue.
