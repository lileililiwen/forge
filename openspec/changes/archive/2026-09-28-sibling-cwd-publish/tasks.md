# Tasks: sibling-cwd-publish

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map existing publish surfaces: `Publish { project, folder, provider, revision, dry_run }`, `PublishCommands` subcommands, `cmd_publish_provider` flow, fleet `cmd_publish_fleet` classification, and `forge deploy status` projection. Confirm no prior cwd discovery.
- [x] 1.2 Confirm sibling identity files on host: `.project.json: id` present in 70+ siblings, `forge.yaml:project.id` for Forge-native projects, directory basename derivation; sample `alethefy`, `hypora`, `crossify`, `openpanel`, plus 2 non-deployable siblings.
- [x] 1.3 Baseline existing contract tests: `tests/publish_contract.rs`, `tests/publish_queue_status_contract.rs`, `tests/publish_observability_contract.rs`, `tests/inventory_contract.rs`, `src/publish/providers` unit tests — record pass baseline.
- [x] 1.4 Confirm provider boundary: `jenkins-local/adapters/forge-publish-provider.py` is invoked only via Forge provider config (`FORGE_PUBLISH_PROVIDER`, `.forge/providers.yaml`); no sibling calls it directly.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement cwd discovery helper (`src/publish/cwd.rs` or `src/publish/mod.rs`): resolve target dir (explicit `--cwd` or process cwd via `canonicalize`), read `.project.json:id` (valid JSON, non-blank), else `forge.yaml:project.id` (serde_yaml), else directory basename; validate with `validate_project_id`; typed `publish-invalid` naming the cwd/file on failure; no parent-directory walk.
- [x] 2.2 Add `--cwd <PATH>` flag to `Publish` cli enum (`src/main.rs`), with `conflicts_with` kept for `project`/`folder` only (cwd is ignored when either is supplied). Wire discovery into `cmd_publish_provider` fallback when `project.is_none() && folder.is_none()`.
- [x] 2.3 Thread discovered `project_dir`/`project_id` through the existing `cmd_publish_provider` body: provider config path resolution (`FORGE_PUBLISH_PROVIDER_CONFIG` or `<project_dir>/.forge/providers.yaml`), `git_revision` from discovered dir, `validate_revision` 40-hex, `operation_id = publish-{id}-{sha12}`, dry-run `PublishProviderRequest` print, `invoke_provider` + phase evidence persistence, `record_publish_phase`. No duplicated provider logic.
- [x] 2.4 Extend `PublishCommands::help` and top-level `forge publish --help` to document bare invocation (`forge publish [--provider <id>] [--revision <sha>] [--cwd <path>] [--dry-run]` publishes cwd).
- [x] 2.5 Add `sibling-cwd-publish` unit tests for identity source order (`.project.json` beats `forge.yaml` beats basename), precedence (`--folder`/`--project` bypass cwd), basename validation, missing/malformed `.project.json` typed refusal, `--cwd` override, non-git revision refusal.
- [x] 2.6 Add `tests/sibling_cwd_publish_contract.rs` CLI contract tests: bare dry-run discovers cwd, `--cwd` variant, precedence over cwd, missing identity failure, non-hex revision refusal, disabled-provider refusal, bare publish persists and is visible via `deploy status --project <id>`, fleet/inventory surfaces unchanged.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove `forge publish --project` and `forge publish --folder` keep byte-equivalent semantics (explicit modes bypass cwd discovery; existing `publish_contract` suites unchanged).
- [x] 3.2 Prove `forge publish fleet --inventory <path>` and `forge inventory show <path>` unchanged: every inventory entry still classified `compose_ready`/`compose_missing`/`invalid`/`source_unavailable`; fleet still serial; `jenkins-local` plugin still invoked through provider contract.
- [x] 3.3 Prove `forge deploy status --project <id>` and `--queue <id>` projections still carry `revision`/`build_status`/`run_status`/`container_identity` for bare publishes (queue_id=None single, same as `--folder`).
- [x] 3.4 Prove no new persistence columns/tables; `operations` `publish` rows for bare and explicit publishes share the same schema; `serde(default)` unaffected.
- [x] 3.5 Prove sibling self-publish from `/tmp` scratch checkout with minimal `.project.json` + `docker-compose.yml`: `forge --registry <tmp-db> publish --dry-run` from sibling cwd succeeds and prints correct `project_id`/`revision`; `forge publish` without flags in empty `/tmp` id-less directory fails typed.

## 4. Verification

- [x] 4.1 Run `cargo fmt --check`, `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain` (full suite green; new supervised suites included).
- [ ] 4.2 Run `cargo test --lib -- --exact generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` when toolchain available (separately recorded).
- [x] 4.3 Run `node scripts/check-openspec-change-names.mjs` and `openspec validate --all --strict --no-interactive` (42 passed, 0 failed); `git diff --check` pass.
- [ ] 4.4 Optional live round-trip: when `jenkins-local/adapters/forge-publish-provider.py` is present on host, bare `forge publish --dry-run` enumerates the same `capabilities` contract as `publish --folder` (skipped honestly when absent).
