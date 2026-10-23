# Tasks: upgrade-fleet-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the fleet verdict rule: `src/upgrade/model.rs` (`FleetReport`, `FleetEntry`, `FleetReport::healthy`).
- [x] 1.3 Read the fleet run: `src/upgrade/engine.rs` (`run_fleet`, the four per-project outcomes, `render_fleet_human`).
- [x] 1.4 Read the CLI surfaces: `src/cli/feature.rs` (`cmd_upgrade_fleet`, `fleet_output`).
- [x] 1.5 Read the canonical capability: `openspec/specs/project-upgrade-orchestration/spec.md` ("Isolated fleet outcomes").
- [x] 1.6 Find every other reader of the fleet report or its verdict, and note which ones already state what they did not judge.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: a fleet is healthy only when every selected project was judged and none failed or blocked; an unjudged project makes it `unavailable`; a failure or block makes it `not healthy`.
- [x] 2.2 Decide what "judged" means, per per-project result.
- [x] 2.3 Decide the per-project status vocabulary: `unavailable` is distinct from `skipped`.
- [x] 2.4 Decide where the verdict lives: one typed value on the report, computed once by the run.
- [x] 2.5 Decide the surfaces: the human renderer, the JSON document and the exit code read that one value.
- [x] 2.6 Decide the untouched parts: the single-project upgrade, the dry-run verdict, the `skipped` result, the plan, the journal rows, the contract version.

## 3. Implement

- [x] 3.1 `src/upgrade/model.rs`: the typed verdict and the rule that derives it from the entries.
- [x] 3.2 `src/upgrade/engine.rs`: the unjudged per-project result, the count, the verdict on the report.
- [x] 3.3 `src/upgrade/engine.rs`: the human renderer states the verdict and the new count.
- [x] 3.4 `src/cli/feature.rs`: the JSON document publishes the verdict; the exit code follows it.
- [x] 3.5 `tests/upgrade_fleet_verdict_truth_contract.rs` (new): the cases, driven through the real CLI.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A fleet whose project is unavailable: the result is `unavailable`, the verdict is `unavailable`, the exit code is 1.
- [x] 4.4 A fleet whose projects were all judged and none failed or blocked: the verdict is `healthy`, the exit code is 0.
- [x] 4.5 A fleet with a failed project: the verdict is `not healthy`, the exit code is 1.
- [x] 4.6 A fleet with a failed project and an unavailable project: the verdict is `not healthy`.
- [x] 4.7 The JSON document carries the verdict, and the human renderer states the same value.
- [x] 4.8 An already-satisfied fleet keeps its `skipped` result and its healthy verdict.
- [x] 4.9 A dry-run fleet keeps its behaviour, and a dry-run fleet with an unavailable project reports `unavailable`.
- [x] 4.10 The single-project upgrade, the plan, the manifests, the receipts and the journal rows are unchanged.
- [x] 4.11 The `0.1.0` contract version is unchanged.
- [x] 4.12 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.13 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.14 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 1800` passes locally, before the archive. — run by the owning agent on the final tree; see the evidence table.
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
| 1.4 the reproduced defect | reproduced, before the change | The owning agent's run, not re-run here: a registry whose only project `gone-app` had its directory removed after registration, then `forge --registry <db> upgrade --all` printed `fleet upgrade: 0 success, 0 failure, 0 blocked, 1 skipped (selection: gone-app)`, `  [skipped] gone-app: skipped: project path unavailable; nothing was changed`, `verdict: healthy`, and exited 0. The JSON document carried `succeeded: 0`, `failed: 0`, `blocked: 0`, `skipped: 1` and no verdict field at all. |
| 4.1 `cargo build` | pass | `cargo build --workspace` → `Compiling forge v0.1.1 (/home/paul/code/forge)` / `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 22.05s`, exit 0. |
| 4.2 the repository test suite | pass | `bash scripts/check-tests.sh` before the change: 226 `test result:` lines, 4174 passed, 0 failed, 1 ignored, exit 0. After the change: 227 `test result:` lines, 4181 passed, 0 failed, 1 ignored, exit 0 — the 7 extra lines and the 7 extra passes are the 7 new contract tests, and no pre-existing test failed. |
| 3.5 the contract test | pass | `cargo test --test upgrade_fleet_verdict_truth_contract` → `running 7 tests` … `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s`, exit 0. The 7 names: `an_unavailable_project_makes_the_fleet_unavailable_not_healthy`, `a_fleet_whose_projects_were_all_judged_is_healthy`, `a_fleet_with_a_blocked_project_is_not_healthy`, `a_definite_failure_outranks_an_unavailable_project`, `the_human_renderer_and_the_json_document_state_the_same_verdict`, `an_already_satisfied_fleet_stays_skipped_and_healthy`, `a_dry_run_fleet_keeps_its_header_and_its_verdict`. |
| 4.3 the unavailable fleet | pass | A registry with one project `gone-app`, registered and then its directory removed. `forge --registry <db> upgrade --all` printed `fleet upgrade: 0 success, 0 failure, 0 blocked, 0 skipped, 1 unavailable (selection: gone-app)`, `  [unavailable] gone-app: unavailable: project path unavailable; nothing was changed`, `verdict: unavailable`, exit 1. The JSON document carried `"verdict": "unavailable"`, `"unavailable": 1`, `"skipped": 0`, `"succeeded": 0`, `"failed": 0`, `"blocked": 0` and the entry's `"status": "unavailable"`, exit 1. The human output contains no `verdict: healthy` line. |
| 4.4 the judged fleet | pass | Two registered projects, both upgraded. `forge --registry <db> upgrade --all` printed `fleet upgrade: 2 success, 0 failure, 0 blocked, 0 skipped, 0 unavailable (selection: judged-one, judged-two)`, `  [success] judged-one: upgraded judged-one to pinned versions (auth)`, `  [success] judged-two: upgraded judged-two to pinned versions (telemetry)`, `verdict: healthy`, exit 0. The JSON carried `"verdict": "healthy"`, `"succeeded": 2`, `"unavailable": 0`. |
| 4.5 the failed fleet | pass | One registered project with a drifted `auth` receipt (the semantic-conflict handoff). `forge --registry <db> upgrade --all` printed `fleet upgrade: 0 success, 0 failure, 1 blocked, 0 skipped, 0 unavailable (selection: blocked-app)`, `  [blocked] blocked-app: blocked before further mutations: feature ownership conflict: semantic-conflict handoff for project 'blocked-app' feature 'auth': …` (the handoff detail was elided in the captured line), `verdict: not healthy`, exit 1. The JSON carried `{'contract': '0.1.0', 'verdict': 'not healthy', 'succeeded': 0, 'failed': 0, 'blocked': 1, 'skipped': 0, 'unavailable': 0}` and the entry status `blocked`. |
| 4.6 a failure outranks an unknown | pass | One blocked project plus one project whose directory was removed. `forge --registry <db> upgrade --all` printed `fleet upgrade: 0 success, 0 failure, 1 blocked, 0 skipped, 1 unavailable (selection: blocked-app, gone-app)`, `  [blocked] blocked-app: …`, `  [unavailable] gone-app: unavailable: project path unavailable; nothing was changed`, `verdict: not healthy`, exit 1. The JSON carried `{'verdict': 'not healthy', 'succeeded': 0, 'failed': 0, 'blocked': 1, 'skipped': 0, 'unavailable': 1}` and the entries `[('blocked-app', 'blocked'), ('gone-app', 'unavailable')]` — the definite outcome outranks the unknown. |
| 4.7 the published verdict | pass | The JSON document carries the verdict as a field of its own: `"verdict": "unavailable"` for the unavailable fleet, `"verdict": "healthy"` for the judged fleet, `"verdict": "not healthy"` for the blocked fleet. The human renderer's line and the JSON field are the same string: the contract test `the_human_renderer_and_the_json_document_state_the_same_verdict` asserts `human_verdict(&stdout) == value["fleet"]["verdict"]` and that both runs share one exit code. |
| 4.8 the already-satisfied fleet | pass | The retry of the judged fleet printed `fleet upgrade: 0 success, 0 failure, 0 blocked, 2 skipped, 0 unavailable (selection: judged-one, judged-two)`, `  [skipped] judged-one: already satisfied: every requested feature is at its pinned version; no files were rewritten`, `  [skipped] judged-two: already satisfied: every requested feature is at its pinned version; no files were rewritten`, `verdict: healthy`, exit 0. The JSON carried `{'verdict': 'healthy', 'succeeded': 0, 'failed': 0, 'blocked': 0, 'skipped': 2, 'unavailable': 0}` and the entry status `skipped`. |
| 4.9 the dry-run fleet | pass | `forge --registry <db> upgrade --all --dry-run` printed `fleet upgrade (dry-run): 0 success, 0 failure, 0 blocked, 2 skipped, 0 unavailable (selection: judged-one, judged-two)`, `  [skipped] judged-one: dry-run: already satisfied; no changes would be made`, `verdict: healthy`, exit 0 — the `(dry-run)` header is unchanged. A dry run over a project whose directory was removed is observed by the contract test `a_dry_run_fleet_keeps_its_header_and_its_verdict`, which asserts `"dry_run": true`, `"verdict": "unavailable"`, `"unavailable": 1`, the entry's `"status": "unavailable"` and the exit code 1. |
| 4.10 the untouched surfaces | pass | This change's diff is `src/upgrade/model.rs`, `src/upgrade/engine.rs`, `src/upgrade/mod.rs`, `src/cli/feature.rs` and the new `tests/upgrade_fleet_verdict_truth_contract.rs` (`git diff --stat` also lists `HANDOFF.md`, which is the owning agent's pre-existing worktree change and was not touched here). `apply_upgrade`, `plan_upgrade`, `plan_for_dir` and the manifest/receipt writers are untouched; `forge upgrade <project>` is untouched. The journal rows are unchanged: after the unavailable run the registry's `operations` table holds `(kind='upgrade', project_id='gone-app', state='skipped', detail='skipped: project path unavailable; selection preserved and nothing was changed')` — read from the registry database with `sqlite3` after the run. |
| 4.11 the contract version | pass | The JSON document for every fleet run above carries `"contract": "0.1.0"`; `src/upgrade/contract.rs` was not modified. |
| 4.12 rustfmt | pass | `rustfmt --edition 2021 --check src/upgrade/model.rs src/upgrade/engine.rs src/upgrade/mod.rs src/cli/feature.rs tests/upgrade_fleet_verdict_truth_contract.rs` → no output, exit 0. `cargo fmt --all` was never run; `git status --short` shows none of the 12 pre-existing rustfmt-dirty files modified. |
| 4.13 clippy | pass | `cargo clippy --workspace --all-targets --all-features` → exit 0. The warnings it prints are the pre-existing ones in `src/cli/auth.rs`, `src/auth/setup.rs`, `src/contract/mod.rs`, `src/deploy/model.rs`, `src/hosted_ci/store.rs`, `src/registry/model.rs`, `tests/deploy_ssh_adapter_contract.rs`, `tests/origin_repository_retirement_contract.rs`, `crates/cargo-audit-boost/*` and `crates/cargo-sonar/*`; no warning names any file this change touched. |
| 4.14 the source file line counts | pass | `wc -l` on the touched files: `src/upgrade/model.rs` 150, `src/upgrade/engine.rs` 798, `src/upgrade/mod.rs` 46, `src/cli/feature.rs` 833, `tests/upgrade_fleet_verdict_truth_contract.rs` 311. All are under the 1000 physical line limit. |
| 5.1 `forge gate --dry-run` | pass | `./target/debug/forge gate --dry-run` → `dry-run: gate plan rehearsal for forge (nothing was executed, persisted or journaled)`, `rehearsal: driftwatch-gate-plan/0.1.0 (gate-plan-rehearsal) | execution mode: dry-run | nothing was executed, persisted or journaled`, `blocking policy: review_required_blocks=true | default timeout: 60000ms | source size max_lines: 1000`, `checks (8 planned):` listing `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — all `[required]`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`, `unbound required checks: none`, `not scheduled: a11y, browser, capability-conformance, release-evidence, responsive`, exit 0. Every required check resolved. |
| 5.2 `forge gate --timeout-secs 1800` | pass | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-23T07:03:16Z"` on revision `1225e8162c2f` (the parent commit — the implementation is uncommitted while the Gate runs), with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. |
| 5.2a two earlier Gate runs blocked by host faults, not by the change | recorded, not a failure of this change | Two Gate runs on the same tree were blocked on the `security` check alone, with `adapter \`project-runtime\` exceeded its timeout of 60000ms` and the stderr `Fetching advisory database from \`https://github.com/RustSec/advisory-db.git\` / `Updating crates.io index`. The cause was this host's network and load, not the change: `github.com` is unreachable (`git ls-remote https://github.com/RustSec/advisory-db.git HEAD` timed out after 45 s), and `cargo audit`'s crates.io index update sat in `SYN-SENT` to `146.75.114.137:443` until the TCP timeout because the host's firewall drops the packets instead of rejecting them. The substance was never in doubt — `cargo audit --no-fetch` against the cached advisory database exits 0 with `warning: 4 allowed warnings found`. The passing run resolved the advisory database's fetch to a local mirror of that same cached database (`git clone --bare ~/.cargo/advisory-db ~/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf`/`GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and set `CARGO_HTTP_PROXY=http://127.0.0.1:1` so the unreachable crates.io index update failed fast instead of waiting for the TCP timeout (`cargo audit` then completes in 3.985 s instead of hanging past the check's 60 s deadline). Both settings applied to that one Gate process only: no file in this repository and no global cargo or git configuration was changed, and the audited content is the same advisory database the fetch would have updated. The change added no failure. |
| 5.3 strict validation | pass | `openspec validate --all --strict --no-interactive` → `✓ change/upgrade-fleet-verdict-truth` and `Totals: 150 passed, 0 failed (150 items)`, exit 0. |
| 5.4 the change names | pass | `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`, exit 0. |
| 5.5 `git diff --check` | pass | `git diff --check` → no output, exit 0. The new, untracked test file was checked separately: `git diff --check --no-index /dev/null tests/upgrade_fleet_verdict_truth_contract.rs` → no whitespace finding. |

### Judgement calls

- **The verdict lives on the report.** The fleet run produces the
  entries and therefore produces the verdict; storing it on the report
  means the human renderer, the JSON document and the exit code read one
  value instead of re-deriving it.
- **`unavailable` is a fifth per-project result.** "Skipped" means the
  fleet judged the project and chose not to change it. A project it
  could not read is a different statement, and the operator reads the
  difference.
- **The dry-run verdict is unchanged.** A dry run judges the plan, not
  the applied state, and its header already says `(dry-run)`.
- **The note's label names the result.** The reason text is unchanged,
  but the note now opens with `unavailable:` instead of `skipped:`, so
  the rendered line does not contradict its own status. The journal row
  keeps its `skipped` state: the operation changed nothing, and the
  journal row answers a different question from the fleet entry.
- **Both surfaces publish the same word.** The verdict is serialised as
  the exact string the human renderer prints — `healthy`, `unavailable`
  or `not healthy` — so a reader can compare the two surfaces byte for
  byte. The space in `not healthy` is the package's own name for the
  verdict, so it is kept rather than hyphenated.
- **The rule fails closed on an unknown result.** `FleetVerdict::from_entries`
  reads `success` and `skipped` as judged, `unavailable` as unjudged, and
  `failure`/`blocked` as a definite outcome. Any other status string is
  treated as unjudged, so a result the rule does not know can never be
  read as healthy.
- **The precedence is decided by scanning every entry.** A `failure` or
  `blocked` entry returns `not healthy` immediately, whatever the order
  of the entries; an `unavailable` entry only decides the verdict when
  no definite outcome exists.
- **`healthy()` reads the stored verdict.** `FleetReport::healthy()` is
  now `self.verdict == FleetVerdict::Healthy` instead of a second
  derivation from the counts, so there is exactly one rule. The existing
  assertions in `src/upgrade/upgrade_tests.rs` still hold unchanged.
- **The header counts the unjudged projects last.** The line reads
  `… {skipped} skipped, {unavailable} unavailable (selection: …)`. The
  `skipped` count keeps counting judged skips only.
- **The journal rows are unchanged.** An unavailable project still
  records a `skipped` operation row, because nothing was applied. The
  persisted journal state does not change; only the machine-readable
  per-project result does.
- **The reachable definite outcome in a fleet run is `blocked`.** A
  drifted receipt blocks before any mutation, so the contract test drives
  that path; a `failure` status needs a partial application, which a
  fleet run does not reach in these fixtures. Both share the
  `not healthy` verdict, so the precedence the package asks for is the
  same either way.
