# Proposal: Define deployment promotion and onboard the first deployable batch

## Why

`deployment/projects.json` is documented as the safe handoff to
jenkins-local, but the manifest is empty: every `.project.json` declares
`deployable: false`, and the adoption queue holds 18+ structurally-adopted
products "non-deployable until their declared verification commands and
release evidence are run and reviewed." Jenkins-side changes now exist that
consume this manifest (jenkins-local `manifest-driven-deployment-eligibility`
and its gate stack) — consumed fail-closed, an empty manifest means nothing
can deploy. The portfolio needs the missing definition: objective promotion
criteria, an auditable promotion record, a script that applies them, and a
first real batch through the door.

## What Changes

- Define promotion criteria in README + a new `profiles`-adjacent policy
  note: a project becomes `deployable: true` only when (1) the checker
  passes for it, (2) its verification command has at least one recorded
  passing execution at a named revision, (3) `jenkins_job` and
  `compose_file` are declared and the compose file exists, (4) the owner
  confirms production configuration will be provisioned.
- Add `scripts/promote_project.py`: runs the per-project check, verifies
  the compose file, writes the `deployment.*` block into that project's
  `.project.json` (non-destructive: refuses to overwrite conflicting
  manual values without `--force`), and appends an entry to a new
  append-only ledger `deployment/promotions.json`
  (`project_id`, `decided_at`, `actor`, `revision`, `evidence_ref`).
- Add checker rule: `deployable: true` without a ledger entry is an ERROR
  (`DEPLOYMENT_PROMOTION`), so the flag can never be hand-edited into
  existence.
- Promote the pilot batch — the second-batch adopted products that have a
  primary `docker-compose.yml` and a real verification command:
  `alethefy`, `crossify`, `cvunify`, `hermexa`, `hypora`, `lexora`,
  `openaccounting` — via the script, after one recorded passing
  verification run each.
- Regenerate and commit `deployment/projects.json`; update README
  (§Jenkins), ROADMAP Phase 4 status, adoption queue and HANDOFF evidence
  text as documentation consequences.

## BFS Impact Map

- **Capabilities:** `deployable-batch-onboarding` (new); consumes the
  existing registry, checker and manifest generator; feeds the
  jenkins-local eligibility gate (external consumer, no code coupling).
- **Users and flows:** project owners approving production onboarding;
  operators running `project.sh` against promoted ids; agents running
  adoption in queue order.
- **Contracts/data/persistence:** new `deployment/promotions.json` ledger;
  manifest schema unchanged (entries now become non-empty); sibling
  `.project.json` files change exactly within the `deployment.*` block.
- **Integrations/configuration:** Jenkins paths in the ledger are the
  project names jenkins-local already uses; no credentials, hosts or
  runtime state enter the registry.
- **Callers:** `workspace_check.py` (new rule), `jenkins_manifest.py`
  (unaffected logic, non-empty output), tests in `tests/`, CI workflow
  unchanged.
- **Failure/boundary behavior:** promotion without evidence ref → script
  refusal; ledger present but declaration later hand-reverted → checker
  ERROR; promote on a project whose compose file is missing → refusal;
  re-promotion is idempotent (UNCHANGED ledger append suppressed).
- **Tests:** promote/ledger/checker-rule matrix on synthetic project trees
  including conflict and revert cases; manifest regeneration golden test.
- **Dependencies:** ordered prerequisite of the jenkins-local
  eligibility change; both can be authored in parallel, but jenkins-local
  removes its migration bypass only after the first batch appears here.
- **Compatibility/security/privacy:** previously non-deployable behavior is
  preserved for every project not promoted; ledger records names, shas and
  pointers only.

## Capabilities

- `deployable-batch-onboarding`: eligibility to deploy becomes an earned,
  recorded, machine-checked state rather than a manual JSON edit.

## Non-goals

- Promoting learning, scratch or archived directories.
- Jenkins-side gate or evidence changes (owned by jenkins-local).
- Automatic or scheduled promotion.
- Changing the manifest schema or profile requirements.
