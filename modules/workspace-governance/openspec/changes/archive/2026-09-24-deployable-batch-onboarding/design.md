# Design: Promotion is a recorded decision, checked every audit

## Ownership

Governance owns *who may deploy*; jenkins-local owns *what actually
deploys*. Promotion is therefore a governance-side transaction: evidence
in, declaration + ledger out, manifest regenerated. The script executes no
project tests itself — it records an evidence reference produced by the
project's own verification command (local run or Jenkins evidence pointer),
keeping the evidence boundary the README already declares.

## promote_project.py contract

```text
python3 scripts/promote_project.py <project-id> \
  --evidence-ref "<revision-or-build pointer>" \
  [--compose-file <name>] [--jenkins-job <name>] [--actor <name>] [--force]
```

Steps (all fail-closed, no partial writes):

1. Load registry entry; refuse unknown/unadopted ids.
2. Run the existing per-project audit; any ERROR → abort naming findings.
3. Detect compose file in the standard order unless
   `--compose-file` overrides; file must exist.
4. Require non-empty `--evidence-ref`; default
   `--jenkins-job`/`--compose-file` to the detected values.
5. Mutate only `deployment.{deployable,jenkins_job,compose_file}` in the
   project's `.project.json`. If those keys already hold different
   non-default values → refuse without `--force` (human-intent
   preservation, same spirit as init_project's non-destructive rule).
6. Append ledger entry (atomic rewrite of `deployment/promotions.json`
   with full prior content preserved; append-only validator refuses
   edits/removals of existing entries on read).
7. Re-run the per-project audit; abort and roll both files back if the
   project now errors.
8. Print the manifest regeneration command as the required next step.

## Ledger

`deployment/promotions.json`:

```json
{"schema": "promotions/1", "entries": [
  {"project_id": "…", "decided_at": "…Z", "actor": "…",
   "revision": "<git sha of project at promotion>",
   "evidence_ref": "…", "manifest_generated": true}
]}
```

The checker rule (ERROR `DEPLOYMENT_PROMOTION` for `deployable: true`
without a current ledger entry for the same project id) makes the ledger
the proof of the flag; a hand-edit that sets `deployable` fails the audit
and therefore fails the Jenkins gate chain.

## Checker integration

Added inside the existing deployable-validation block: after the
compose-file and jenkins-job checks pass, look up the ledger. Missing
entry → ERROR. Entry with a different `revision` than the project's git
HEAD → WARNING `DEPLOYMENT_PROMOTION_STALE` (evidence points at an older
tree; promotion is still a valid human decision, staleness is triage
information, never a silent block).

## Batch execution

Pilot batch = the seven second-batch products with `docker-compose.yml`
present and real verification commands. Each requires one owner-or-agent
recorded passing verification run before `promote_project.py` is executed;
runs and refs are listed in this change's tasks so the promotion commits
carry the evidence trail in-repo. Learning/scratch projects (hestiaLab,
loreMix directory-name mismatches) are explicitly excluded from the batch
and remain on the queue.

## Rollback of eligibility

`promote_project.py --demote <id>` (or manual edit + ledger note) restores
`deployable: false`; ledger keeps the history with a `demoted_at` field.
Jenkins-side jobs for demoted projects become `ORPHAN_UNMANIFESTED` reports
in jenkins-local — governance never deletes them.

## Verification strategy

`tests/test_promote_project.py` on synthetic trees (full success matrix,
conflict, missing compose, missing evidence ref, demote/re-promote,
ledger-append-only validator); checker-rule fixtures;
`jenkins_manifest.py --check` after regenerating with the batch promoted;
`workspace_check.py --self-check`. No Mac access required or attempted.
