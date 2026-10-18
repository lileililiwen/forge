# Tasks: Define deployment promotion and onboard the first deployable batch

## 1. BFS — Baseline and impact coverage

- [ ] Map every existing consumer of `deployment.*` fields
  (`workspace_check.py` rules, `jenkins_manifest.py` filter, jenkins-local
  manifest consumers) before adding the ledger rule.
- [ ] Confirm each pilot-batch project (`alethefy`, `crossify`,
  `cvunify`, `hermexa`, `hypora`, `lexora`, `openaccounting`) has a real
  verification command and a primary `docker-compose.yml`; record any
  exclusions with reasons.
- [ ] Add fixtures/skeletons for promote, demote, ledger-append and
  checker-rule tests before implementing.
- [ ] Verify proposal/design/spec agree on the promotion criteria wording.

## 2. DFS — Requirement-by-requirement implementation

- [ ] Implement `scripts/promote_project.py` (audit → compose detect →
  evidence-ref requirement → declaration mutation → ledger append →
  re-audit rollback) with `--force` and `--demote`.
- [ ] Implement the append-only `deployment/promotions.json` validator.
- [ ] Add checker rules `DEPLOYMENT_PROMOTION` (ERROR) and
  `DEPLOYMENT_PROMOTION_STALE` (WARNING).
- [ ] Execute one recorded passing verification run per pilot project and
  capture its ref (list command + exit result per project in this file's
  notes as the evidence trail).
- [ ] Run `promote_project.py` for each pilot project; regenerate
  `deployment/projects.json`.
- [ ] Update README §Jenkins, ROADMAP Phase 4 status, adoption queue batch
  notes and HANDOFF next-action as documentation consequences.

## 3. BFS — Cross-surface regression and completeness

- [ ] Re-run `workspace_check.py --self-check`, the full unittest suite and
  `jenkins_manifest.py --check`; non-promoted projects must audit exactly
  as before.
- [ ] Hand-edit one fixture project to `deployable: true` without a ledger
  entry and confirm the ERROR surfaces in `--audit` output and blocks
  `jenkins_manifest.py`.
- [ ] Confirm manifest entries carry the declared
  `verification_command`/`compose_file` jenkins-local consumes, and that
  demotion removes an entry cleanly.
- [ ] Verify no credential, host-path or Mac-runtime data entered
  registry, ledger or manifest.

## 4. Verification

- [ ] `python3 -m unittest discover -s tests -v`.
- [ ] `python3 scripts/workspace_check.py --self-check` and
  `python3 scripts/jenkins_manifest.py --check`.
- [ ] `node scripts/check-openspec-change-names.mjs` and strict
  `openspec validate` for this change.
- [ ] Record actual outputs; the runtime effect on the Mac fleet is
  jenkins-local-side observation and is named as the next action there,
  never claimed here.
