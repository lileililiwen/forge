# Tasks: cross-repo-adoption-grant

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map every requirement in `proposal.md` onto the existing code
  paths: `validate_project()` (`scripts/workspace_check.py:1096-1105`), the
  `finding()` severity vocabulary, the `auto_gate.py` error-only fail rule
  (`scripts/auto_gate.py:36`), and the promotion-ledger validation in
  `scripts/promote_project.py` that this ledger pattern is copied from.
- [x] 1.2 Add the `adoption_grant` and `declines` shapes to
  `schemas/project.schema.json` as additive properties. Confirm the 81
  existing declarations remain valid with neither block present, and that
  `$defs.capability.required` is untouched.
- [x] 1.3 Add fixtures before implementation: one `adoption_grant` valid
  fixture per permitted write class (declaration, manifest, wiring, test,
  `AGENTS.md`); one invalid fixture per forbidden write class (provider
  source, provider version bump, cross-repo `ProjectReference`, sibling
  `AGENTS.md`, promotion ledger); grant lifecycle fixtures (issued,
  consumed, expired, revoked, single-use); and ledger fixtures (hand-edited
  record, hand-added record, rollback row).
- [x] 1.4 Record the fleet baseline: 81 declarations, none carrying an
  `adoption_grant` block, and the finding set each must keep producing
  unchanged.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `scripts/validate_adoption_grant.py` logic to
  `workspace_check.py` and call it from `validate_project()`. Parse the
  block, require `capability`, `provider`, `package`, `actor`, `issued_at`,
  `expires_at`, and `write_set`; emit `ADOPTION_GRANT_SHAPE` on a malformed
  block and validate nothing further.
- [x] 2.2 Implement identity resolution: `provider` must be a registered
  project id and `capability` must already be bound to that provider in the
  capability registry. A grant that names an unbound capability is refused —
  the grant may not create the binding it consumes. Evidence: a fixture
  naming an unregistered provider and one naming an unbound capability, both
  refused without a write.
- [x] 2.3 Implement the window check: `expires_at` in the past is
  `ADOPTION_GRANT_STALE`; a consumed or revoked grant is `ADOPTION_GRANT_STALE`
  for any second repoint. Evidence: a single-use test proving a second
  repoint under a consumed grant is refused.
- [x] 2.4 Implement the provider check: the provider's declared capability
  must be `verified` with a resolving `evidence_ref`, else
  `ADOPTION_GRANT_PROVIDER_UNVERIFIED`. Evidence: a fixture with a
  `configured` provider and an unresolvable `evidence_ref`.
- [x] 2.5 Implement the write-set resolution: every path matched by
  `write_set` must resolve inside the consumer root. A `..` or absolute
  pattern is `ADOPTION_GRANT_SHAPE`; a resolved path outside the root, or
  inside the provider root, is `ADOPTION_GRANT_SCOPE_VIOLATION` and the file
  is not read.
- [x] 2.6 Implement the forbidden-set checks: provider-root writes, provider
  version bumps, `ProjectReference`/relative/`../` dependencies on a sibling
  checkout, `deployment/promotions.json` writes, and `deployable: true`
  promotion are all `ADOPTION_GRANT_SCOPE_VIOLATION` regardless of the
  resulting code state. Evidence: one fixture per forbidden class, each
  asserting ERROR.
- [x] 2.7 Add `adoption/adoptions.json` as an append-only ledger modeled on
  `deployment/promotions.json`: `schema_version`, `records[]`, and a
  `content_hash` over each record body. Record creation refuses a
  hand-edited record whose hash does not match
  (`ADOPTION_GRANT_UNSATISFIED`) and a hand-added record that violates the
  append discipline.
- [x] 2.8 Implement the record-completeness check: a consumer declaring
  `capabilities[].consumes[]` for a granted capability must have a matching
  adoption record, else `ADOPTION_GRANT_UNSATISFIED`. Record the six
  mandatory fields — `replaced`, `provider`/`package`/`version`, `actor`,
  `grant_expires_at`, `rollback_ref`, `recorded_at` — and implement rollback
  as a second `kind: "rollback"` row that never edits or deletes the first.
- [x] 2.9 Confirm `auto_gate.py` requires no change:
  `ADOPTION_GRANT_SCOPE_VIOLATION` is an ERROR, so the existing
  `summary["errors"]` rule already fails the unattended gate, and the report
  schema is unchanged.
- [x] 2.10 Amend the shared `AGENTS.md` clause per `design.md` §9. Deliver
  it through the existing `rules-baseline` single-source mechanism; do not
  hand-edit the 9+ repositories that carry the clause. The amendment splits
  *source* edits to a sibling (still forbidden) from *consumption* of a
  sibling's published artifact (permitted under a grant), and adds the
  requirement to name the specific sibling, project id, or package assessed.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove the fleet baseline is unchanged: all 81 declarations produce
  exactly the findings they produced before, and a clean fleet reports zero
  new ERRORs.
- [x] 3.2 Prove the grant cannot be laundered into a provider change: a
  grant naming `0.1.0` that is then used to justify `0.2.0` is
  `ADOPTION_GRANT_SCOPE_VIOLATION`, and no permitted write class can reach
  the provider root by path, symlink, or manifest indirection.
- [x] 3.3 Prove the audit is read-only: a recursive tree hash of a fixture
  consumer is identical before and after, no child process is spawned, and
  no file outside the governance root is written.
- [x] 3.4 Confirm the boundaries this grant does not touch:
  `registry-integrity` values (`id`, `path`, `adoption`, `lifecycle`) are
  unchanged; `deployment/promotions.json` is not extended, reused, or
  modified; no `deployable` transition is reachable; no `gate.yaml` is
  changed. Evidence: the existing registry, promotion, and
  `test_forge_governance_adapter.py` suites pass unchanged.
- [x] 3.5 Confirm the Forge adapter is untouched: the new findings appear
  only in the existing `findings` list, with no observation key added,
  removed, or renamed, and exit codes 0/2/3 unchanged.

## 4. Verification

- [x] 4.1 Run the governance suite plus the new grant tests:
  `python3 -m unittest discover -s tests -v`, and confirm no existing test
  changed expectation.
- [x] 4.2 Run `python3 scripts/workspace_check.py --self-check` and
  `python3 scripts/workspace_check.py --root .. --audit --json`, then
  record the new WARNING/ERROR breakdown honestly.
- [x] 4.3 Amend `docs/shared-ownership.md` (or its canonical successor) with
  the source-vs-consumption split, the grant shape, the forbidden set, and
  the five `ADOPTION_GRANT_*` codes. Add a pointer from `README.md`'s
  capability section so the `AGENTS.md` clause and the governance contract
  cannot drift apart. Keep `docs/vocabulary.md` as the single entry point.
- [x] 4.4 Update the `workspace-governance` skill reference and run
  `python3 scripts/check_skill_refs.py` so every newly cited path exists.
- [x] 4.5 Run `openspec validate cross-repo-adoption-grant --strict` and
  `openspec list`, and record both outputs.

## 5. Explicit deferred work (not in this cycle)

- [ ] 5.1 Issuing a grant by CLI (`scripts/grant_adoption.py`) or by signed
  token. This cycle specifies the declaration and its checks; how a human or
  agent obtains one is an owner decision recorded in the proposal.
- [ ] 5.2 The first adoption executed under the grant. The grant is the
  permission; each adoption is a separate, individually reviewed change with
  its own rollback.
- [ ] 5.3 Consolidating the 58 local `WebApplicationFactory` subclasses
  across 25 repos onto the shared 98-line factory, and the 12 hand-rolled
  Postgres test fixtures onto `Platform.Testing`. Both are follow-on work
  and each needs its own grant.
- [ ] 5.4 Granting adoption for the `tests: true` gate check — 76 of 76
  repos declare it, 6 bind a command, and `driftwatchdog` returns
  `NotApplicable`/Info. Owned by a separate change in the `driftwatchdog`
  repository. Referenced here, not duplicated; the `gate.yaml` check
  vocabulary was read to confirm the boundary and no `gate.yaml` is changed.
- [ ] 5.5 Merging the three incompatible "capability" vocabularies
  (`platform-contracts`, the declarative block, hermora's endpoint list).
  Owned by `platform-contracts`; this change treats the capability id as an
  already-registered value and never creates one.
- [ ] 5.6 Tightening the "name your assessment" requirement into a checked
  finding. This cycle amends the rule text only; a check that parses
  `AGENTS.md` prose across repos belongs to the shared-rules drift tooling
  (`scripts/rules_drift.py`), not here.
