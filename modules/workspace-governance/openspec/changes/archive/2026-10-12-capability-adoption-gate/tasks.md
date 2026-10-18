# Tasks: capability-adoption-gate

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map every requirement in `proposal.md` onto the existing code
  paths: `validate_capabilities()` (`scripts/workspace_check.py:332`),
  `validate_project()` (`:1096-1105`), the `CAPABILITY_OWNERS` /
  `CAPABILITY_EVIDENCE_STATES` constants (`:84-85`), the `finding()`
  severity vocabulary, and the `auto_gate.py` error-only fail rule
  (`scripts/auto_gate.py:36`).
- [x] 1.2 Confirm the additive shape change: add `consumes` to
  `$defs.capability` in `schemas/project.schema.json` without touching the
  existing `required: ["owner", "evidence_state"]`, and confirm
  `validate_capabilities()` is called with the consumer project path already
  available so no new plumbing is needed.
- [x] 1.3 Add schema fixtures before implementation:
  `consumes[]` valid, `consumes[]` missing `evidence_ref`, `consumes[]`
  `evidence_ref` escaping the root, `consumes[]` non-object entry;
  `declines[]` valid, `declines[]` unknown rationale, `declines[]`
  unresolvable `evidence_ref`.
- [x] 1.4 Record the no-regression baseline: 79 of 81 declarations carry no
  `consumes[]`, and 2 carry a `capabilities` block
  (`forge` 7 entries, `workspace-governance` 4). Every one of those 81
  must produce the same finding set as before this change.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `capabilities.json` at the governance root with the
  capability-id → provider binding, `posture`, `expected_since`,
  `audit_count`, and an append-only `graduations` list. Follow the
  `decisions.json` pattern: missing, unparseable, or empty is a governance
  ERROR with no empty-registry fallback. Do not modify `projects.json`.
- [x] 2.2 Add the bounded, read-only manifest reader
  (`Directory.Packages.props`, `*.csproj`, `package.json`, `Cargo.toml`,
  `pyproject.toml`, `requirements*.txt`). Parse dependency stanzas only.
  Apply the existing `vocabulary.json` secret-substring and `..`/absolute
  path rules. Evidence: a test proving no child process is spawned and that
  a `..` `evidence_ref` is refused without being read.
- [x] 2.3 Implement `measure_capability_adoption()` as the three-way join
  (registry posture × manifest edge × `consumes[]`) and call it from
  `validate_project()` after `validate_capabilities()`. Emit
  `CAPABILITY_ADOPTION_MISSING` at WARNING for `expected` posture and ERROR
  for `required`, with the provider, the package, and any declared local
  implementation path in the message.
- [x] 2.4 Add `CAPABILITY_ADOPTION_UNEVIDENCED` for a manifest-proven edge
  with no `consumes[]` record. Evidence: fixtures for the five repos that
  already pin `Platform.*` in `Directory.Packages.props`
  (`therapist-commons`, `tantalyn`, `careBridge`, `citylens`, `devloom`)
  assert WARNING and never `CAPABILITY_ADOPTION_MISSING`.
- [x] 2.5 Add `CAPABILITY_ADOPTION_EVIDENCE_STALE` when a
  `consumes[].evidence_ref` stops resolving inside the consumer root.
- [x] 2.6 Add the decline vocabulary and
  `CAPABILITY_ADOPTION_DECLINE_UNRECORDED`. The closed rationale set is
  `ownership`, `privacy`, `lifecycle`, `dependency-direction`,
  `naming-symmetry`, `not-applicable`, `provider-unavailable`. A decline
  without a resolving `evidence_ref` is itself a finding. Evidence: one
  fixture per rationale, including `dependency-direction` grounded in
  `therapist-commons/.ai-rules/workflow.md:22-25` and `naming-symmetry`
  grounded in `openTeacherKit/README.md:36-39`.
- [x] 2.7 Implement the ramp. `posture` is `exempt` | `expected` |
  `required`; a capability is always admitted as `expected`; promotion to
  `required` requires a `verified` provider with a resolving `evidence_ref`,
  `graduation_audits` consecutive recorded audits, and an appended
  graduation entry. Evidence: a test proving a capability cannot reach
  `required` by lowering a counter or waiting — only a recorded entry moves
  it, and the entry requires the provider evidence first.
- [x] 2.8 Add `CAPABILITY_ADOPTION_UNRESOLVED` at REVIEW_REQUIRED for a
  capability id that does not bind to exactly one provider. Assert it never
  escalates to ERROR regardless of posture.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove the fleet baseline is unchanged for non-adopting repos: no
  new ERRORs, and the 81 declarations produce the same findings as the
  pre-change baseline. The check is additive at the same level as the
  `product_code_quality.py` opt-in.
- [x] 3.2 Prove a decline is repo-local and capability-scoped: a decline in
  one fixture repo never suppresses the same capability's finding in another
  fixture repo, and never suppresses a different capability in the same repo.
- [x] 3.3 Prove the provider is never charged for its own capability: a
  registry entry whose `provider` equals the audited repo id resolves to
  `exempt`, not a self-adoption failure.
- [x] 3.4 Confirm `maturity_audit.py` still reports the single
  `workspace_check.py` result and never computes a second opinion, and that
  the Forge adapter's observation keys, exit codes (0/2/3), and redaction
  behaviour are untouched. Evidence: the existing adapter mapping-matrix
  test in `tests/test_forge_governance_adapter.py` passes unchanged and the
  new findings appear only in the existing `findings` list.
- [x] 3.5 Confirm `auto_gate.py` requires no change: a `required`-posture
  ERROR already fails the unattended gate via the existing
  `summary["errors"]` rule, and the report schema is unchanged.

## 4. Verification

- [x] 4.1 Run the governance suite plus the new adoption tests:
  `python3 -m unittest discover -s tests -v`, and confirm no existing test
  changed expectation.
- [x] 4.2 Run the read-only proof on the real fleet:
  `python3 scripts/workspace_check.py --self-check` and
  `python3 scripts/workspace_check.py --root .. --audit --json`, then
  record the new WARNING breakdown honestly. A recursive tree hash of a
  fixture project must be identical before and after, and no child process
  may be spawned.
- [x] 4.3 Update `docs/capabilities.md` with the `consumes[]` shape, the
  posture table, the decline vocabulary, and the five new finding codes;
  update `README.md` so the capability section states that adoption is
  measured. Keep `docs/vocabulary.md` as the single entry point and link
  rather than restate.
- [x] 4.4 Update the `workspace-governance` skill reference and run
  `python3 scripts/check_skill_refs.py` so every newly cited path exists.
  The skill routing table and `references/platform-adoption.md` are updated and
  cite `capabilities.json`, `docs/capabilities.md`, `docs/adoption-grant.md`,
  and `adoption/adoptions.json`. After `cross-repo-adoption-grant` landed,
  `python3 scripts/check_skill_refs.py` passed: `skill refs ok: SKILL.md + 9
  references`.
- [x] 4.5 Run `openspec validate capability-adoption-gate --strict` and
  `openspec list`, and record both outputs.

## 5. Explicit deferred work (not in this cycle)

- [ ] 5.1 Merging the three incompatible "capability" vocabularies — the
  `platform-contracts` contract family, the declarative block, and hermora's
  endpoint list — into one id space. Owned by `platform-contracts`; this
  change ships `CAPABILITY_ADOPTION_UNRESOLVED` in its place.
- [ ] 5.2 Behavioural proof of adoption (a registered shared wrapper is
  actually exercised) rather than a pinned consumption coordinate. The
  evidence bar is an owner decision and is recorded in the proposal.
- [ ] 5.3 Rewriting `kind`, which conflates role and stack and misclassifies
  `forge`/`driftwatchdog`/`ariadex` against their own declarations. Findings
  in this change are keyed on `provider` id, never on `kind`.
- [ ] 5.4 Registering the four active unregistered repos
  (`argoscope`, `chronicleaf`, `devloom`, `kairovia`) so they can be
  adoption-scored. `DISCOVERED_UNREGISTERED` remains their finding.
- [ ] 5.5 The `tests: true` gate-command binding — 76 of 76 repos declare
  it, 6 bind a command, and `driftwatchdog` returns `NotApplicable`/Info.
  Owned by a separate change in the `driftwatchdog` repository. Referenced
  by this change, not duplicated. The gate check vocabulary and `gate.yaml`
  shape were read to confirm the boundary; no `gate.yaml` is changed here.
- [ ] 5.6 Granting the remediation permission. An agent that may legally
  repoint a consumer onto a provider is the separate
  `cross-repo-adoption-grant` change. This change measures and records
  regardless of whether that grant exists.
