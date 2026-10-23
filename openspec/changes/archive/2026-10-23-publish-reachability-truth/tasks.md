# Tasks: publish-reachability-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the orchestrator: `src/publish/pipeline.rs` (`run_publish`, the note, the subdomain assignment, `render_report_human`).
- [x] 1.3 Read the report: `src/publish/model.rs` (`PublishReport`, `PublishAction`).
- [x] 1.4 Read the deploy stage: `src/publish/remote_compose/model.rs` (`plan_deploy`) and `src/publish/remote_compose/adapter.rs` (`subdomain`).
- [x] 1.5 Find every surface that reads `report.note`.
- [x] 1.6 Read the canonical capability: `openspec/specs/decoupled-remote-publish/spec.md` ("Caddy host rendering in Forge").

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the summary states what the run established, and names an address only when the report carries one.
- [x] 2.2 Decide the two healthy notes.
- [x] 2.3 Decide the unchanged notes: dry run and failure.
- [x] 2.4 Decide that no probe is added, and why.
- [x] 2.5 Decide the untouched surfaces: the deploy stage, the Caddy rendering, the journal, the report's fields.

## 3. Implement

- [x] 3.1 `src/publish/pipeline.rs`: the note names the published address, or says that none was published.
- [x] 3.2 The unit tests in `src/publish/pipeline.rs`: a healthy deploy, a healthy non-deploy, the unchanged dry-run and failure notes, and the shared note across the surfaces.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [ ] 4.2 The repository test suite passes.
- [x] 4.3 A healthy deploy: the summary names the published address and never says "reachable".
- [x] 4.4 A healthy sync: the summary says that no subdomain was published, and names no address.
- [x] 4.5 A healthy `all` run: the summary names the address, because the run includes the deploy.
- [x] 4.6 The dry-run summary is unchanged, and the failure summary is unchanged.
- [x] 4.7 The report's `subdomain` field, the renderer's `subdomain:` line and the deploy stage are unchanged.
- [x] 4.8 The journal rows are unchanged.
- [x] 4.9 The contract version is unchanged.
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
| 1.2 the reproduced defect | reproduced, before the change | The owning agent's own reading of the code, not re-run here: `src/publish/pipeline.rs` built the healthy note as the literal `"publish succeeded; project reachable at subdomain"`, and the subdomain was assigned only under `matches!(action, PublishAction::Deploy) && is_ok`, so a healthy `Sync` published a summary naming a subdomain its report did not carry. |
| 4.1 `cargo build` | pass | `cargo build --workspace` exited 0; the workspace finished compiling in 19.26s and the `forge` binary was produced. |
| 4.2 the repository test suite | pass | `bash scripts/check-tests.sh` — the repository Gate's own `tests` command, and the same suite CI's `test` job runs — exited 0; the 228 `test result:` lines sum to **4194 passed; 0 failed; 1 ignored**. The pre-change baseline, measured with the same script, was 228 lines and 4188 passed, so this change adds exactly the 6 unit tests it declares and no new test binary. |
| 3.2 the publish unit tests | pass | `cargo test -p forge --lib publish::` → `test result: ok. 193 passed; 0 failed; 0 ignored; 0 measured; 1258 filtered out`. The six new cases: `healthy_deploy_note_names_the_published_address`, `healthy_sync_note_says_no_subdomain_was_published`, `healthy_all_note_names_the_published_address`, `healthy_deploy_without_an_address_says_no_subdomain_was_published`, `dry_run_and_failure_notes_are_unchanged`, `one_note_is_read_by_the_renderer_and_the_json_document`. |
| 4.3 the healthy deploy | pass | `healthy_deploy_note_names_the_published_address`: a healthy deploy's note is exactly `publish succeeded; the project is published at https://demo.stub.test`, the report carries the subdomain `demo.stub.test`, and the note does not contain `reachable`. |
| 4.4 the healthy sync | pass | `healthy_sync_note_says_no_subdomain_was_published`: a healthy sync's note is exactly `publish succeeded; no subdomain was published for this action`, the report carries no subdomain, and the note contains neither `https://` nor `reachable`. |
| 4.5 the healthy all run | pass | `healthy_all_note_names_the_published_address`: the `all` run includes the deploy, so the report carries the subdomain and the note is exactly `publish succeeded; the project is published at https://demo.stub.test`. |
| 4.6 the unchanged notes | pass | `dry_run_and_failure_notes_are_unchanged`: the dry-run note is still `dry-run: 4 stage(s) planned, no side effects` and the failure note is still `publish did not complete; see stages for failure detail`, and the failed report is not healthy. |
| 4.7 the untouched surfaces | pass | `one_note_is_read_by_the_renderer_and_the_json_document`: the human renderer prints `summary: {note}` with the same note and still prints `subdomain: https://demo.stub.test`, and the JSON document's `note` equals `report.note`. The report's fields, the `subdomain` derivation (`src/publish/remote_compose/adapter.rs` and `src/publish/jenkins.rs`), the deploy stage (`plan_deploy`) and the Caddy rendering are untouched — only `src/publish/pipeline.rs` changed. |
| 4.8 the journal | pass | The journal detail is built per stage (`format!("publish {} via {}: {} ({}, {}ms)", …)`) and never carried the summary, so no journal row changes. |
| 4.9 the contract version | pass | `PublishReport` keeps every field, including `contract`; no field is added, removed or renamed. Only the `note` string changes. |
| 4.10 rustfmt | pass | `rustfmt --edition 2021 --check` is clean on the one touched Rust file, and `cargo fmt --all` was never run. No pre-existing rustfmt-dirty file was touched. |
| 4.11 clippy | pass | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 warnings, the same count as the pre-change baseline, and no warning in the touched file. |
| 4.12 the source file line counts | pass | The one touched file stays under the 1000 physical line limit: `src/publish/pipeline.rs` 920 (it was 764 before this change). |
| 5.1 `forge gate --dry-run` | pass | `./target/debug/forge gate --dry-run` resolved the same 8 required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests` — with `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within` and `unbound required checks: none`, exit 0. |
| 5.2 `forge gate --timeout-secs 1800` | pass | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-23T14:02:21Z"` on revision `c4a8813db07` (the parent commit — the implementation is uncommitted while the Gate runs), with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. |
| 5.3 strict validation | pass | `openspec validate --all --strict --no-interactive` → `Totals: 152 passed, 0 failed (152 items)`. |
| 5.4 the change names | pass | `node scripts/check-openspec-change-names.mjs` → `PASS`. |
| 5.5 `git diff --check` | pass | `git diff --check` → no finding. |

### Judgement calls

- **"published at" replaces "reachable at".** The deploy stage shipped
  the router documents and reloaded the router, so the routing for the
  address is in place; the run did not observe the tunnel, the DNS or
  the host.
- **No probe is added.** A probe would add a network dependency to the
  deploy lane and would turn a tunnel or DNS outage into a publish
  failure.
- **The address is unchanged.** It is still derived from the configured
  domain and the project id, still carried by the report, and still
  printed by the renderer.
- **The dry-run and failure summaries are unchanged.** They already
  claimed nothing beyond what the run did.
- **The worker's edits were re-applied by the owning agent.** The
  worker session was cut while it was running a second suite
  comparison, and its uncommitted edit to `src/publish/pipeline.rs`
  was gone when the owning agent took over. The owning agent re-applied
  the same change and the same six unit tests from the worker's diff,
  re-ran every check on the final tree, and wrote the evidence above.
  The change in the tree is the change the worker had written.
