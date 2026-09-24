# Tasks: Make the deploy plane executable through the Jenkins adapter

## 1. BFS — Baseline and impact coverage

- [x] Re-verify the exact argv Forge's `src/deploy` passes today and freeze
  it as contract 0.1.0 with the envelope schema.
- [x] Map jenkins-local's `project.sh`/`deploy-all.sh` verbs and status
  output shapes into the adapter mapping table.
- [x] Identify callers (CLI, provider matrix, API route, journal) and the
  MCP exclusion to re-verify.
- [x] Add stub-adapter fixtures for every status and failure class.

## 2. DFS — Requirement-by-requirement implementation

- [x] Publish `docs/adapter-contracts/deploy-executor.md`.
- [x] Implement `adapters/jenkins/forge-deployer-jenkins` reference
  adapter (dry-run/apply/observe mapping, explicit health table).
- [x] Wire contract-version checks into the deploy runner's parse step.
- [x] Update the provider matrix `deploy` row semantics to the adapter.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-verify existing deploy contract tests with the default binary
  path unchanged.
- [x] Re-verify `--confirm`, partial-stage, stale-identity and redaction
  boundaries; prior-state preservation on failure.
- [x] Re-verify MCP `tools/list` still omits deploy and API route behavior
  unchanged.
- [x] Confirm no host paths or secrets persist outside `.forge/` config
  references.

## 4. Verification

- [x] `cargo fmt --all -- --check`; `cargo build`; `cargo test
  --all-targets`; `cargo clippy --all-targets -- -D warnings`.
- [x] Adapter shellcheck/python lint per repo norms (record the chosen
  command).
- [x] `node scripts/check-openspec-change-names.mjs`; `openspec validate
  --all --strict --no-interactive`; `git diff --check`.
- [x] Dry-run the reference adapter against fixtures; live Jenkins evidence
  explicitly deferred to the jenkins-local adoption checklist.
