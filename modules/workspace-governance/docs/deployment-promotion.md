# Deployment Promotion

A project becomes `deployable: true` only through a recorded promotion
transaction. The script `scripts/promote_project.py` is the only path
that flips the flag; hand-edits fail the audit with `DEPLOYMENT_PROMOTION`
once the promotion rule is in place.

## Promotion criteria

A project is ready for promotion when:

1. The governance audit passes for the project (no `ERROR` findings on
   its current state, i.e. `deployable: false`).
2. A compose file (`docker-compose.yml` or `compose.yml`) is present at
   the project root, unless `--compose-file` overrides.
3. The operator supplies a non-empty `--evidence-ref`. The reference is
   a pointer to where the verification command's last passing run was
   recorded (a commit SHA, a Jenkins build URL, a `local:HEAD:<sha>`
   marker, etc.). The script never runs the verification itself.
4. The post-audit (with `deployable: true` set) also passes. In
   particular, the project must carry a complete `release_evidence`
   block: each required field (`revision`, `artifacts`, `digests`,
   `provenance`, `checks`, `publication`) is either `verified` or
   `configured` with a present `evidence_ref` file. If the post-audit
   fails, the script rolls the declaration and the ledger back
   atomically — no partial state is observable.

## Promotion script

```text
python3 scripts/promote_project.py <project-id> \
  --evidence-ref "<revision-or-build pointer>" \
  [--compose-file <name>] [--jenkins-job <name>] \
  [--actor <name>] [--force]

python3 scripts/promote_project.py <project-id> --demote
```

The script:

1. Runs the per-project audit; any `ERROR` on the current state aborts.
2. Detects the compose file (or honors `--compose-file`); refuses if
   missing.
3. Requires a non-empty `--evidence-ref`.
4. Refuses to overwrite a manually-declared `compose_file` or
   `jenkins_job` that differs from the detected/default values without
   `--force`. The default state of `(false, null, null)` is not a
   conflict.
5. Mutates only the `deployment.{deployable,jenkins_job,compose_file}`
   keys in `.project.json`.
6. Appends an entry to `deployment/promotions.json` and updates the
   tamper-evident `deployment/promotions.sha256` sidecar.
7. Re-runs the audit; rolls both files back on any new `ERROR`.

Re-promotion of the same project at the same revision with the same
evidence reference is a no-op (the ledger is not appended).

## Promotion ledger

`deployment/promotions.json` is an append-only JSON document:

```json
{
  "schema": "promotions/1",
  "entries": [
    {
      "project_id": "alethefy",
      "decided_at": "2026-09-24T07:52:33.506272+00:00",
      "actor": "workspace-governance/batch",
      "revision": "da625855334e3431118be6ccdaa78e351feb8465",
      "evidence_ref": "local:HEAD:da625855334e3431118be6ccdaa78e351feb8465",
      "manifest_generated": true,
      "kind": "promote",
      "compose_file": "docker-compose.yml",
      "jenkins_job": "alethefy"
    }
  ]
}
```

`kind` is `promote` or `demote`. Both kinds coexist in the ledger for
history; demotions never delete a prior promote entry.

A `deployment/promotions.sha256` sidecar records the SHA-256 of the
serialized `entries` array. Every load compares the file's entries
against the sidecar; a mismatch raises `PROMOTIONS_LEDGER_TAMPERED`
(`ERROR`) on the next audit and refuses further appends. The only way
to update the sidecar is for the script to write a new ledger through
its append path.

## Audit rules

| Code                          | Level   | Meaning                                                                                |
|-------------------------------|---------|----------------------------------------------------------------------------------------|
| `DEPLOYMENT_PROMOTION`        | ERROR   | `deployable: true` without a matching `promote` entry in the ledger.                    |
| `DEPLOYMENT_PROMOTION_STALE`  | WARNING | The ledger's `revision` differs from the project's current `HEAD`; triage, not blocking. |
| `PROMOTIONS_LEDGER_TAMPERED`  | ERROR   | The on-disk ledger does not match its sidecar; the audit refuses to read or append.    |

The `RELEASE_EVIDENCE_INSUFFICIENT` and `RELEASE_EVIDENCE_REQUIRED` rules
(unchanged) still block promotion when the project is missing or has
incomplete `release_evidence` fields.

## Demotion

`promote_project.py --demote <id>` flips `deployable` back to `false`
and appends a `demote` entry. The previous `promote` entry remains in
the ledger for history. The audit immediately accepts the demoted state
unless other rules fire.

## Pilot batch

The first batch of promoted projects is recorded in the ledger with
`actor: workspace-governance/batch` and `evidence_ref:
local:HEAD:<revision>`. Each project's `.project.json` carries a
`release_evidence` block in `configured` state pointing at pilot-context
placeholder artefacts (`release-artifacts/*.txt`); the placeholder
content names the pilot context, the source revision at promotion, and
the project's verification command. Real artefacts replace the
placeholders when the project's release pipeline runs.
