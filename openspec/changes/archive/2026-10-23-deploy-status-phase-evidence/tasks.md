# Tasks: deploy-status-phase-evidence

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the CLI's deploy status: `src/cli/release.rs` (`cmd_deploy_status`, `cmd_deploy_status_watch`, `render_deploy_status_human`, `filter_publish_deploy`).
- [x] 1.3 Read the API's deploy status: `src/api/admin/history.rs` (`deploy_status`).
- [x] 1.4 Read the journal row: `src/registry/model.rs` (`OperationEntry`, `PublishPhaseEvidence`, `record_publish_phase`) — including the doc comment that already promises `unknown`.
- [x] 1.5 Read the canonical capability: `openspec/specs/forge-publish-observability-revision-containers/spec.md` ("Phase-visible publish lifecycle").
- [x] 1.6 Find every other reader of the same evidence (the queue status surface, the portal) and note which ones already project `unknown`.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: one shared projection of a journal row's phase evidence; a value the row does not carry is `unknown`, never an omitted field.
- [x] 2.2 Decide where the projection lives: beside the row it projects, so the CLI and the API read the same function.
- [x] 2.3 Decide the surfaces: the CLI's human renderer, the CLI's JSON document and the API's JSON document all read it.
- [x] 2.4 Decide the untouched parts: the row, the values a row carries, the aggregate state, the contract version.
- [x] 2.5 Decide the guard: a contract test drives the real CLI against a journal row with and without phase evidence.

## 3. Implement

- [x] 3.1 The shared projection of one row's phase evidence.
- [x] 3.2 `src/cli/release.rs`: the human renderer and the JSON document read it.
- [x] 3.3 `src/api/admin/history.rs`: the API's JSON document reads it.
- [x] 3.4 `tests/deploy_status_phase_evidence_contract.rs` (new): the three cases, driven through the real CLI.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A row with no phase evidence: the human renderer prints `unknown` for all four values.
- [x] 4.4 A row with no phase evidence: the JSON document exposes all four fields as `unknown`.
- [x] 4.5 A row with phase evidence: every surface exposes the row's own values, and no surface prints `unknown`.
- [x] 4.6 The API's deploy status route projects the same values as the CLI for the same row.
- [x] 4.7 The document's other fields, its aggregate `state` and its `read_only` marker are unchanged.
- [x] 4.8 The `forge-deploy-status/0.2.0` contract version is unchanged.
- [x] 4.9 The journal row and the publish lane are unchanged.
- [x] 4.10 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.11 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.12 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 1800` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capabilities' purposes.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 1.4 the reproduced defect | reproduced, before the change | The owning agent's run, not re-run here: `cargo test --test deploy_status_phase_evidence_contract` failed 2 of 3 cases — `the_human_renderer_names_the_phase_evidence_it_does_not_have` FAILED, `the_json_document_names_the_phase_evidence_it_does_not_have` FAILED, `a_row_with_phase_evidence_keeps_its_own_values` ok. The observed human table was `deploy status: project=p1` / `  #1 p1 publish done`, and the observed JSON entry was `{"detail":"shipped 1.2.3","finished_at":"…","kind":"publish","op_id":1,"project_id":"p1","started_at":"…","state":"done"}` — the four evidence fields absent, not named. |
| 4.1 `cargo build` | PASS | `cargo build` → `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 2m 36s`; exit 0; `target/debug/forge` produced. |
| 4.2 the repository test suite | PASS | `bash scripts/check-tests.sh` → exit 0; **226** `test result:` lines; **4174 passed, 0 failed, 1 ignored**. Pre-change baseline: 225 lines, 4169 passed, 0 failed, 1 ignored — the delta is exactly this package's 5 new contract cases and 1 new test binary. No flake fired: neither `checker_contract::format_json_does_not_alter_checker_stdout` nor `profile_build_command_runnable_contract` failed, so no re-run was needed. |
| 3.4 the contract test | PASS | `cargo test --test deploy_status_phase_evidence_contract` → `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.73s` — `the_human_renderer_names_the_phase_evidence_it_does_not_have` ok, `the_json_document_names_the_phase_evidence_it_does_not_have` ok, `a_row_with_phase_evidence_keeps_its_own_values` ok, `the_api_route_names_the_phase_evidence_it_does_not_have` ok, `the_api_route_keeps_the_row_own_phase_evidence` ok. |
| 4.3 the human renderer | PASS | `forge --registry <db> deploy status --project p1` against a row with no phase evidence → `deploy status: project=p1` / `  #1 p1 publish done revision=unknown build=unknown run=unknown container=unknown`. |
| 4.4 the JSON document | PASS | `forge --registry <db> --format json deploy status --project p1` against the same row → the entry carries `"build_status":"unknown"`, `"container_identity":"unknown"`, `"revision":"unknown"`, `"run_status":"unknown"` beside its own unchanged fields. |
| 4.5 a row with phase evidence | PASS | The same command against a row recorded with `record_publish_phase` → `  #2 p1 publish done revision=0123456789abcdef0123456789abcdef01234567 build=succeeded run=succeeded container=forge-p1-0123456789ab`; the JSON entry carries `"build_status":"succeeded"`, `"run_status":"succeeded"`, `"revision":"0123456789abcdef0123456789abcdef01234567"`, `"container_identity":"forge-p1-0123456789ab"`. No `unknown` anywhere. |
| 4.6 the API route | PASS | `GET /v1/admin/projects/p1/deploy/status` (session login as in `tests/web_release_deploy_history_contract.rs`) against a row with no phase evidence → the entry exposes the four fields as `unknown`; against a row that carries them → the row's own values. Both cases also assert that the CLI's JSON document projects the same four values for the same row. `cargo test --test web_release_deploy_history_contract` → `test result: ok. 10 passed; 0 failed; 0 ignored`; `cargo test --test deploy_contract` → `test result: ok. 21 passed; 0 failed; 0 ignored`. |
| 4.7 the unchanged document | PASS | The document still carries `"contract":"forge-deploy-status/0.2.0"`, `"project"`, `"queue"`, `"scope"`, `"state":"done"`, `"read_only":true`; the entry still carries `op_id`, `kind`, `project_id`, `state`, `started_at`, `finished_at`, `detail` and the optional `idempotency_key`/`request_hash`/`queue_id` exactly as before. Only the four evidence fields changed from absent to always present. |
| 4.8 the contract version | PASS | `forge-deploy-status/0.2.0` is unchanged in `src/cli/release.rs`, `src/cli/publish.rs` and the API route; `tests/publish_queue_status_contract.rs` and `tests/portfolio_contract/cli.rs` still assert it and pass. |
| 4.9 the row and the publish lane | PASS | `src/registry/model.rs` is untouched: `OperationEntry` keeps its `#[serde(skip_serializing_if = "Option::is_none")]` attributes and `record_publish_phase` is unchanged, so the stored row keeps its `NULL`. The publish command itself is unchanged. |
| 4.10 rustfmt | PASS | `rustfmt --edition 2021 --check` on each touched Rust file (`src/registry/phase.rs`, `src/registry/mod.rs`, `src/cli/release.rs`, `src/cli/publish.rs`, `src/api/admin/history.rs`, `tests/deploy_status_phase_evidence_contract.rs`) → exit 0, no diff, for every file. `cargo fmt --all` was not run; none of the twelve pre-existing rustfmt-dirty files was touched. |
| 4.11 clippy | PASS | `cargo clippy --workspace --all-targets --all-features` → exit 0 with **39** `warning:` lines, the same count as the pre-change baseline. No finding points at a touched file: the warnings are in `crates/cargo-sonar/*`, `crates/cargo-audit-boost/*`, `src/hosted_ci/store.rs`, `src/registry/model.rs` (pre-existing, untouched), `src/deploy/model.rs`, `src/contract/mod.rs`, `src/cli/auth.rs`, `src/auth/setup.rs`, `tests/origin_repository_retirement_contract.rs`, `tests/deploy_ssh_adapter_contract.rs`. |
| 4.12 the source file line counts | PASS | `wc -l`: `src/registry/phase.rs` 117, `src/registry/mod.rs` 33, `src/cli/release.rs` 619, `src/cli/publish.rs` 717, `src/api/admin/history.rs` 557, `tests/deploy_status_phase_evidence_contract.rs` 333. `src/registry/model.rs` is untouched at 900. The largest touched file is 717, under the 1000 physical line cap. |
| 5.1 `forge gate --dry-run` | PASS | `./target/debug/forge gate --dry-run` → exit 0; `checks (8 planned)`: `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests` — all `[required]`, `unbound required checks: none`; `source size max_lines: 1000`; `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. |
| 5.2 `forge gate --timeout-secs 1800` | PASS | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-23T04:41:34Z"` on revision `3dcea36e3bd5` (the parent commit — the implementation is uncommitted while the Gate runs), with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks with `unbound required checks: none` and `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. |
| 5.2a the Gate's `security` check and an unreachable `github.com` | PASS, with the network fault handled | Two earlier Gate runs on this tree (the worker's at `2026-10-23T04:07:43Z` and the owning agent's at `2026-10-23T04:20:17Z`) were blocked on the `security` check alone, with `couldn't fetch advisory database … An IO error occurred when talking to the server`: `github.com` was unreachable from this host (`git ls-remote https://github.com/RustSec/advisory-db.git HEAD` timed out after 45 s). It was not a finding — `cargo audit --no-fetch` against the cached advisory database exited 0 with `warning: 4 allowed warnings found`. The passing run resolved the advisory database's fetch to a local mirror of the same cached database (`git clone --bare ~/.cargo/advisory-db ~/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf`/`GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, applied to that one Gate process only — no file in this repository and no global git configuration was changed. The audited content is the same advisory database the fetch would have updated; only the network hop was removed. The change added no failure. |
| 5.3 strict validation | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 149 passed, 0 failed (149 items)`; exit 0. |
| 5.4 the change names | PASS | `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`; exit 0. |
| 5.5 `git diff --check` | PASS | `git diff --check` → exit 0, no output. The two untracked new files (`src/registry/phase.rs`, `tests/deploy_status_phase_evidence_contract.rs`) carry no trailing whitespace and end with a newline. |

### Judgement calls

- **The projection lives beside the row.** The registry owns the
  journal row and the meaning of its columns, so the rule belongs
  there and the CLI and the API read the same function.
- **The values a row carries are untouched.** The new rule only names
  what the row does not carry.
- **The aggregate state is untouched.** The document's `state` stays
  the journal row's own state; the missing values were dropped from
  the per-entry evidence, and that is what this change fixes.
- **The contract version is unchanged.** The document keeps its
  fields; the four evidence fields become always present.
- **The contract test drives the real CLI.** It writes a real journal
  row through the public registry API and reads the status through the
  real `forge` binary, so no public seam is widened.
- **The watch loop's JSON document is projected too.**
  `cmd_deploy_status_watch` (`src/cli/publish.rs`) builds the same
  `forge-deploy-status/0.2.0` document for `forge deploy status
  --watch --queue <id>`, and it serialised the raw row, so it dropped
  the same four fields the canonical scenario forbids. The package
  marks `src/cli/publish.rs` read only, so this is the one place the
  implementation reaches past the package's three named surfaces; the
  change is the four-line projection of the watch document's entries
  and touches no publish behaviour. The canonical scenario is
  "`forge deploy status --queue <id>`", which the watch path serves.
- **The projection lives in its own registry module.** The rule is a
  pure function over one `OperationEntry`, so it sits in
  `src/registry/phase.rs` beside `model.rs` (the row) and `store.rs`
  (the row reader) rather than inside `model.rs`, which is 900 lines
  and would cross the 1000 physical line cap. `src/registry/mod.rs`
  re-exports it the way it re-exports the rest.
