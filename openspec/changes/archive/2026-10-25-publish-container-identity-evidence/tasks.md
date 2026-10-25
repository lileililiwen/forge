# Tasks: publish-container-identity-evidence

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the provider contract: `src/publish/providers/mod.rs`, `src/publish/providers/model.rs` (`PublishProviderResponse`), `src/publish/providers/engine.rs` (`compose_project_name`).
- [x] 1.3 Read the publish surfaces: `src/cli/publish.rs`, `src/api/handlers_core.rs`, `src/api/admin/publish.rs`, `src/cli/fleet.rs`.
- [x] 1.4 Read the canonical capability: `openspec/specs/forge-publish-observability-revision-containers/spec.md`.
- [x] 1.5 Read the existing tests: `tests/publish_observability_contract.rs`, `tests/sibling_cwd_publish_contract.rs`, `tests/forge_web_project_publish_contract.rs`, `tests/publish_fleet_dry_run_verdict_truth_contract.rs`.
- [x] 1.6 Read the projection: `src/registry/phase.rs` (a missing identity renders as `unknown`).
- [x] 1.7 Reproduce the defect on the current tree and record the row it wrote.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the provider's own identity is the truth, the canonical identity is composed only for a response that published, and a response that published nothing carries no identity.
- [x] 2.2 Decide where the rule lives: the provider contract, read by every publish surface.
- [x] 2.3 Decide the options and reject the ones that keep the unconditional composition or that drop the synthesis the canonical capability authorises.
- [x] 2.4 Decide the untouched surfaces: the fleet lane, the delivery lane, the projection, the provider's own identity, the successful legacy case.

## 3. Implement

- [x] 3.1 `src/publish/providers/mod.rs`: the rule and its completion vocabulary.
- [x] 3.2 `src/cli/publish.rs`: both publish lanes read the rule.
- [x] 3.3 `src/api/handlers_core.rs` and `src/api/admin/publish.rs`: both routes read the rule.
- [x] 3.4 `tests/publish_container_identity_evidence_contract.rs`: the failed row, the successful row, the provider's own identity and the projection.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A response that published nothing carries no container.
- [x] 4.4 A successful response keeps the canonical identity, and a provider's own identity is always kept.
- [x] 4.5 The fleet lane, the delivery lane and the projection are unchanged.
- [x] 4.6 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.7 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.8 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 2400` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the modified capability is promoted.
- [x] 6.2 Update the modified capability's purpose and strip the archive's trailing blank lines by hand.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

### The reproduced defect

```
$ forge --registry /tmp/pub/registry.db publish --folder /tmp/pub/demo \
    --provider legacy --revision 0123456789abcdef0123456789abcdef01234567
# the provider reported: status=failed, health=unhealthy, no container_identity
$ sqlite3 /tmp/pub/registry.db \
    "SELECT state, container_identity FROM operations WHERE kind = 'publish'"
failed|forge-demo-0123456789ab
```

The row said `failed` and named a container for a publish that failed.
The identity was composed by Forge from the revision alone.

The composition was unconditional, in four places:

```rust
// src/cli/publish.rs, before
let container_identity = response.container_identity.clone().unwrap_or_else(|| {
    forge::publish::providers::compose_project_name(project_id, &phase_revision)
});
```

The canonical capability already draws the line the other way — "WHEN a
provider returns a **successful** response without a `container_identity`
field" — and the fleet lane already reads it: "The container identity is
the provider's own. The lane never composes one: a run that published
nothing has no container."

### The change

| Surface | Before | After |
|---|---|---|
| `src/publish/providers/mod.rs` | — | the rule: `proven_container_identity` and `PUBLISH_COMPLETED_STATUSES` |
| `src/cli/publish.rs` (both publish lanes) | composed the identity always | read the rule |
| `src/api/handlers_core.rs` | composed the identity always | read the rule |
| `src/api/admin/publish.rs` | composed the identity always | read the rule |
| `src/cli/fleet.rs` | kept only the provider's own identity | unchanged — the rule agrees with it |

### The checks

| Check | Result | Evidence |
|---|---|---|
| The reproduced defect | reproduced before the change | The row above: `failed|forge-demo-0123456789ab`. The composition was the `unwrap_or_else` in `src/cli/publish.rs`. |
| The contract test fails without the change | PASS | With the rule's completion check removed (so it composes for every response again), `a_response_that_published_nothing_carries_no_container` fails with `left: Some("forge-demo-0123456789ab"), right: None`; the other two cases pass on both trees, as they should. |
| A response that published nothing carries no container | PASS | `a_response_that_published_nothing_carries_no_container`: the row's state is `failed`, its `container_identity` is `None`, and the status projection renders `unknown`. |
| A successful response keeps the canonical identity | PASS | `a_successful_response_keeps_the_canonical_identity`: the row carries `forge-demo-0123456789ab` and the projection surfaces it. |
| The provider's own identity is always kept | PASS | `the_providers_own_identity_is_always_kept`: a failed response that reports `forge-fixture-0123456789ab` keeps exactly that identity. |
| One rule, every publish surface | PASS | The four publish surfaces call `proven_container_identity`; the file `src/publish/providers/mod.rs` owns the decision. No other place composes an identity: `grep` finds `compose_project_name` only in the rule, in the fleet lane's absence of it, and in the contract's own tests. |
| The untouched surfaces | PASS | `git diff` touches no line of `src/cli/fleet.rs`, `src/delivery/`, `src/registry/phase.rs` or the provider contract's auto-generated modules. The existing publish suites pass unchanged: `publish_observability_contract` 8 passed, `publish_contract` 11 passed, `sibling_cwd_publish_contract` 12 passed, `forge_web_project_publish_contract` 9 passed, `publish_fleet_dry_run_verdict_truth_contract` 3 passed, `forge_web_publish_fleet_contract` 7 passed. The canonical legacy-provider case — a provider that published, reported no identity and no phase evidence — still gets `forge-demo-0123456789ab`, as the canonical scenario requires. |
| Repository test suite | PASS | `bash scripts/check-tests.sh` — the repository Gate's own `tests` command, and the same suite CI's `test` job runs — exited 0; the 247 `test result:` lines sum to **4290 passed; 0 failed; 1 ignored**. The pre-change baseline was 246 lines and 4287 passed, so this change adds exactly the 3 tests it declares and one new test binary. |
| Repository Gate | PASS, 8/8 | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-25T11:53:02Z"` on revision `a07c3a28f5229d3d36c0a4f1bad6438cffee605e` (the parent commit — the implementation is uncommitted while the Gate runs), with all eight required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — in state `pass`. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. The advisory database's fetch was resolved to the local mirror of the same cached database (`/home/paul/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf` / `GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and `CARGO_HTTP_PROXY=http://127.0.0.1:1` made the unreachable crates.io index update fail fast instead of waiting for the TCP timeout. Both settings applied to that one Gate process only. |
| OpenSpec, change names, whitespace | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 168 passed, 0 failed (168 items)`; `node scripts/check-openspec-change-names.mjs` → `PASS`; `git diff --check` and `git diff --cached --check` → no finding. Archived without `--skip-specs`; the `forge-publish-observability-revision-containers` requirement was modified with all three canonical scenario names kept and one scenario added, and the archive's trailing blank lines were stripped from the canonical spec by hand. `openspec list` reports no active changes. |
| Lint and formatting | PASS | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 lines starting with `warning`, the same count as the pre-change baseline, and no warning names a touched file. `rustfmt --edition 2021 --check` is clean on all five touched Rust files, and `cargo fmt --all` was never run. No pre-existing rustfmt-dirty file was touched. Every touched file stays under the 1000 physical line limit: `src/publish/providers/mod.rs` 59, `src/cli/publish.rs` 752, `src/api/handlers_core.rs` 930, `src/api/admin/publish.rs` 572, `tests/publish_container_identity_evidence_contract.rs` 301. |
| The worker | the owning agent wrote the change | The worker subagents in this programme have been unreliable, so the owning agent wrote this change. The candidate came from the `explore` subagent's ranked list; the owning agent verified it against the canonical scenario, reproduced it, and wrote the rule, the four call sites and the tests. |
