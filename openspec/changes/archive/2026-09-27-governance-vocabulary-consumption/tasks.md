# Tasks: Consume the governance vocabulary instead of re-declaring it

## 1. BFS — Baseline and impact coverage

- [ ] Confirm the companion prerequisite before touching code:
  `workspace-governance/vocabulary.json` exists, its `schema_version`,
  `profiles`, `kinds`, `placeholder_markers` and `secret_field_substrings`
  match the shipped change, and `openspec` shows that change archived. If the
  file is absent this package stays a proposal and no task below is started.
- [ ] Inventory every locally re-declared vocabulary value with source lines:
  `src/generate/workspace.rs` (profile mapping, `const EVIDENCE_STATUS` at
  `:40`), `src/gate/mod.rs:82`, `src/policy/mod.rs:36`,
  `src/provider/mod.rs:84-89` and its re-split at `:567`, `src/profile/mod.rs:98`
  (`WorkspaceMapping`), `src/identity`, and the credential word set at
  `src/policy/mod.rs:932`.
- [ ] Record the portfolio `evidence_status` census on this host
  (`implemented` 41, `verified` 34, `planned` as the template value, one-off
  `mvp-validated`, one prose paragraph) and the fact that
  `schemas/project.schema.json` bounds the field only by `minLength: 1`. File the
  normalization request to Workspace Governance as a follow-up package with its
  dependency edge; do not resolve it inside Forge.
- [ ] Capture the pre-change quality measurement for this repository: run
  `python3 ../workspace-governance/scripts/product_code_quality.py . --json` and
  record the marker count (28 today), the per-file breakdown, the split between
  the words `placeholder` (18) and `stub` (10) versus maintainer-debt markers
  (0 today: `TODO`, `FIXME`, `XXX`, `HACK`, `unimplemented!`, `todo!` have no
  match in `src/`), and confirmation that Rust's lexical rule already passes
  because every `#[test]` sits behind a `#[cfg(test)]` guard.
- [ ] Confirm `workspace_check.py` currently reports zero findings for this
  project and note that `profiles/rust-product.json` does not exist, so
  `load_profile_spec` returns `None` (`workspace_check.py:161-173`, `:463-471`)
  and `CAPABILITY_UNSUPPORTED` cannot fire yet.
- [ ] Map every consumer of the values being switched: `forge new`,
  `forge import`'s `workspace_metadata` observation, `forge doctor`, `forge
  check`, `forge fleet`, the portal project/settings sections, the receipt and
  upgrade refresh path, and the per-profile contract tests that pin declaration
  bytes.

## 2. DFS — Requirement-by-requirement implementation

- [ ] Vendor `vocabulary.json` into `contracts/vocabulary/governance-vocabulary.json`,
  extend `contracts/manifest.json` with its source and revision, extend
  `scripts/sync-contracts.mjs` to cover the governance source, and add the
  offline digest test.
- [ ] Implement vocabulary loading with the documented resolution order
  (`--vocabulary PATH` → `FORGE_GOVERNANCE_VOCABULARY` → vendored file), size
  bounds, traversal rejection, unparseable refusal that does not fall through,
  and an honest `vocabulary-unavailable` state that never blocks a workflow.
- [ ] Move the governance profile mapping into the profile descriptor as data
  and validate every declared `governance_profile` against the consumed
  canonical profile set at descriptor load, refusing a non-canonical value by
  name.
- [ ] Reduce the runtime-name duplication to one consumed/shared source used by
  the gate, policy and provider surfaces, keeping the ordered
  `driftwatchdog` → `driftwatch` probe behaviour and every refusal message
  byte-compatible.
- [ ] Split the secret vocabulary by role in code and in `contracts/vocabulary/README.md`:
  contract field names from `platform-contracts`, repository check words from
  governance, captured-output redaction staying in
  `policy::redact_credentials`.
- [ ] Rename the incidental marker identifiers (`now_placeholder()` at
  `src/ui_pattern/mod.rs:717` and `src/component/mod.rs:425`, the `stub()`
  helper at `src/gate/mod.rs:1070`) to intent-revealing names that do not
  contain a marker word.
- [ ] Add the `quality` block to `.project.json` with `product_roots`,
  `languages`, the explicit `placeholder_markers` override (marker words that
  are Forge domain vocabulary removed, all debt markers retained) and
  `placeholder_threshold: 0`, and bind
  `python3 scripts/product_code_quality.py <dir>` in `.ai-gate/gate.yaml`.
- [ ] Add the `capabilities` block for this repository using only states this
  checkout can honour: `configured` with a real `evidence_ref` per implemented
  capability, `blocked` for `tenancy` and `billing` with the non-goal recorded in
  `.ai-rules/completion.md`, and no entry whose pointer does not resolve.
- [ ] Normalize this repository's own declaration `kind` to the canonical value
  the consumed vocabulary supports (`control-plane` → `platform` per the WG
  remap table) and leave `evidence_status` at the value the vocabulary defines;
  the pending local `planned` → `implemented` edit is excluded from these commits
  until the field is canonicalized.
- [ ] Emit `capabilities` in generated declarations only for descriptors that
  declare them; no speculative entries and no copying of Forge's own set.
- [ ] Implement the `declaration-vocabulary` doctor finding with
  `applicable: false` outside its scope, warn-level divergence, fail only for a
  Forge-authored claim that no longer resolves, redaction, char bounds, and the
  `<project>` path substitution.
- [ ] Project the new finding through the checker plane for opted-in projects
  without changing the alert document schema, and through the portal project and
  settings sections as read-only text.

## 3. BFS — Cross-surface regression and completeness

- [ ] Re-run the per-profile generation matrix and prove the stale-unedited
  refresh path against declarations generated before the switch: refresh names
  both files, re-hashes the receipt, an edited declaration refuses with the
  ownership-conflict code leaving both files byte-preserved, and a foreign
  declaration is never rewritten or blocked on.
- [ ] Re-prove `--no-workspace-metadata` byte-identity against the pinned
  pre-change digests, and confirm the changed declaration remains inert
  (`cargo build`/`cargo test` of a generated tree with it present and absent).
- [ ] Re-run the fleet verbatim tests: a Workspace Governance profile that Forge
  does not know still surfaces unchanged and is never coerced.
- [ ] Re-run the checker byte-identity suite for projects that did not opt in,
  and confirm the new finding never changes a doctor verdict, maturity level or
  exit code for them.
- [ ] Run `python3 scripts/workspace_check.py --project forge --root ..` in
  Workspace Governance and confirm the portfolio oracle stays at zero errors,
  then run the product-code quality checker and record the post-change count.
- [ ] Confirm no Forge workflow gained a runtime dependency on the sibling: with
  `contracts/vocabulary/` deleted, generation, import, doctor, check, fleet and
  gate behave as before and report `vocabulary-unavailable`.
- [ ] Update `README.md`, `docs/architecture.md`, `docs/requirements-coverage.md`
  and `docs/release-readiness.md` for the consumed vocabulary, the marker
  decision and the recorded residual, and record the two open requests
  (`evidence_status` canonicalization, `rust-product` profile spec) as companion
  asks rather than Forge-side inventions.

## 4. Verification

- [ ] `cargo fmt --all -- --check`; `cargo build`; `cargo clippy --all-targets
  -- -D warnings`; `cargo test --all-targets` with the long native-toolchain
  test excluded and then run separately with its recorded duration.
- [ ] `python3 ../workspace-governance/scripts/product_code_quality.py . --json`
  green under the declared override with `placeholder_threshold: 0`, plus the
  recorded pre-change count (28), the renamed identifiers and the residual
  domain-vocabulary matches.
- [ ] `python3 ../workspace-governance/scripts/workspace_check.py --project
  forge --root ..` at zero errors, and the same run against a scratch project
  generated by this build so the emitted declaration and its capability states
  satisfy the sibling's own audit.
- [ ] If the gate runtime is available and its store initialized in the runner,
  run `forge gate .` and record the real verdict including the new quality
  check; no gate pass is claimed for this repository from local evidence.
- [ ] `node scripts/check-openspec-change-names.mjs`; `openspec validate --all
  --strict --no-interactive`; `git diff --check` plus review of added files;
  archive without `--skip-specs` only after every scoped scenario has evidence.
- [ ] Record in `HANDOFF.md` the consumed vocabulary source and revision, the
  states chosen for each capability with their evidence refs, the marker
  decision, and the two companion requests still open in Workspace Governance.
