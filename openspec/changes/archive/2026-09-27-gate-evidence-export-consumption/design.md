# Design: Consuming an executed gate's release evidence

## Ownership and boundaries

| Concern | Owner | This package's role |
| --- | --- | --- |
| The export document: its entry point, fields, per-check → per-field state mapping, stale-revision refusal, unknown-field construction error | Driftwatchdog (`gate-evidence-export`, workspace package 4) | Consumer. Parses, validates, refuses. Never re-decides a state the producer refused to grant |
| Evidence field names and state vocabulary (`revision`…`publication`; `declared`/`configured`/`verified`/`unverified`) | Workspace Governance (`docs/capabilities.md`, `vocabulary.json` via `governance-vocabulary-consumption`) | Consumed, never re-declared |
| Wire shape of a shared release-evidence document (`release_id`, `service`, `version`, `released_at`, `evidence_kind`) | `platform-contracts` `platform.release-evidence`, owned runtime "driftwatchdog (release publisher)" | Validated against the vendored schema through `platform-contract-consumption` |
| Gate verdict, execution, run history, blocking policy | Driftwatchdog, already consumed by `gate-runtime-evidence` | Unchanged. This is an *evidence* view beside the *verdict* view, not a replacement for it |
| Deployment eligibility and promotion | Workspace Governance promotion path, Jenkins-local | None. Consumed evidence never sets `deployable` |
| Publishing, signing, SBOM, provenance production | a release publisher | None. Forge records what a producer asserted and refuses what it did not produce |

The two views stay deliberately separate: `forge gate status` answers "what did
the gate decide at this revision", and `forge gate evidence` answers "what does
that executed run establish as evidence, field by field". One is a verdict, the
other is a claim inventory, and merging them is how a blocked gate starts
reading as evidence of health.

## Inbound document handling

The export is an untrusted artefact by this repository's security rules
(`.ai-rules/concerns/security.md`: treat external findings and provider payloads
as untrusted). Concretely:

1. Size-bounded read, single-document parse, and the same shape-discrimination
   discipline the gate status document already uses — the gate plane learned that
   assumption twice and its fixtures record it in
   `tests/fixtures/gate/NOTES.md`.
2. Contract/version gate: a document naming an unsupported major or an unknown
   family is `unavailable` naming the observed version, never coerced.
3. Vocabulary gate: every field name must be one of the nine WG fields and every
   state one of the four non-`blocked` release states. `blocked` in an evidence
   field is refused, because WG reserves it for capability declarations — a
   producer misusing the vocabulary is reported, not silently normalised.
4. Revision gate: the document's run revision must equal the revision Forge
   captured for the evidence it is being bound to. A mismatch makes **every**
   field `unverified`, mirroring the producer's own refusal rule so the two
   implementations cannot disagree.
5. Attribution gate: a `verified` field must carry an `evidence_ref` that
   resolves inside the project, and the record must name its producer and
   producer revision. A `verified` `publication` with no artifact digest is
   refused as contradictory rather than recorded — the same contradiction rule
   that downgrades a `PASS` document riding a non-zero exit.
6. Redaction and path scrubbing before the record is persisted, journaled or
   printed, with the assessed project's absolute path replaced by `<project>` and
   char bounds enforced.

Anything that fails 1–6 leaves the previously persisted record byte-identical and
journals an honest `failed`/`unavailable` outcome. No failure path may clear or
overwrite good evidence.

## Persistence and journaling

- Consumed evidence lands in the gate plane's existing
  `.forge/gate/<project-id>/` layout as a distinct record beside `evidence.json`,
  written atomically (`.tmp` + rename), revision-bound, producer-attributed.
- The operations journal gains no new kind: it reuses the existing `gate` kind
  with the states already defined there, because a second kind for the same
  executed run would split one event across two rows.
- One latest record per project; history stays in the journal, matching the
  release/deploy/analytics split this repository already uses.
- Read surfaces never journal, and a rehearsal never persists — the two rules
  `gate-runtime-evidence` established and this package inherits.

## The evidence view

```text
ReleaseEvidenceView {
  project_id, producer, producer_revision, run_revision, captured_at,
  freshness: fresh | stale | absent,
  fields: [ { name, state, evidence_ref, attribution } ],
  counts: { verified, configured, declared, unverified, refused },
  refusal_reasons: [ … ]
}
```

- `fresh` requires an exact revision binding, the rule already enforced for gate
  evidence: an unbound revision can never read as fresh.
- `refused` counts entries rejected by rules 3–5 so an operator sees the
  disagreement instead of a quietly shortened list.
- Absent export renders `absent` with the reason named, never an empty field list
  that could be read as "no evidence needed".

## Populating this repository's own block

`forge/.project.json` gains `release_evidence` derived **only** from a real
executed run, which is what makes the claim non-circular:

| Field | Expected state | Why |
| --- | --- | --- |
| `revision`, `version`, `toolchain`, `checks` | `verified` when the export grants it, `unverified` otherwise | these are the fields an executed gate on this checkout can establish |
| `artifacts`, `digests`, `sbom`, `provenance` | `unverified` | `artifact-and-ci-baseline` produces a packaged artifact with digests, but SBOM and provenance production do not exist yet |
| `publication` | `unverified` with a stated non-goal | Forge publishes nothing; `docs/release-readiness.md` records "this package claims no remote publication", and `artifact-and-ci-baseline` explicitly refuses promotion |

Two preconditions that must hold before any write, or the block is not written at
all:

- **This checkout must have an executed gate pass.** It does not. The `.driftwatch`
  store is missing here and the sibling-owned next action from the previous cycle
  is still open, so on this host the honest state is every field `unverified` and
  no `verified` claim anywhere. Writing a `verified` block before the store exists
  would be exactly the hand-written-placeholder failure package 4 exists to
  remove, transplanted into Forge.
- **The declaration must stay Forge-owned and unedited.** The existing ownership
  receipt covers `.project.json`; an edited declaration refuses with the standard
  ownership-conflict code rather than being overwritten, and a foreign
  declaration is never rewritten or blocked on.

`deployable` stays `false`, and the change records why: WG's rule is that
`release_evidence` never promotes a project to deployable on its own, and a
non-deployable project's evidence block is informative, not an eligibility claim.

## No change to generated declarations

A generated project has executed no gate, so it gets **no** `release_evidence`
block — an empty or speculative block would be an assertion without a run. The
`workspace-metadata-emission` shape (no evidence beyond observed absence)
continues unchanged, and its per-profile digest tests stay as the guard that this
package did not leak a claim into a template.

## Doctor and checker projection

- New `release-evidence` finding: `pass` when a declared `verified` state is
  backed by a fresh attributed record for that field; `warn` on stale or
  partially evidenced blocks; `fail` for the specific inconsistency the
  structural audit cannot see — a declaration asserting `verified` in
  `verification.evidence_status` while carrying no `release_evidence` block, or
  naming a field with a reference that does not resolve; `unverified` when no
  export has been consumed.
- Non-gating by construction: `applicable: false` for projects with no
  declaration and no export, so absence never lowers health, never changes a
  maturity verdict and never alters an exit code — the shape `workspace-metadata`
  and `gate-evidence` already use, and the reason the checker plane stays
  byte-identical for non-declared projects.
- `forge check` projects the finding as an alert with the existing severity
  mapping (`fail`→`error`, `warn`/`unavailable`/`unverified`→`warning`) and the
  `doctor/<id>` symbol, with no change to the emitted document schema.
- The provider matrix gains no row: this consumes a runtime that already has its
  own `gate-runtime` row, and a second row for the same boundary would double a
  claim about the same binary.

## Release wiring

The release plane's `gate` check kind may cite consumed evidence at the captured
revision, extending the existing rule that a release check can never cite stale
evidence: a `gate`-kind check now resolves through the consumed record, and if
the record is absent, stale or refused the stage is `unavailable` and the plan is
not ready. No new check kind, no new state vocabulary, no re-decision of what the
gate concluded.

## Failure boundaries

| Condition | Behaviour |
| --- | --- |
| No export produced | `absent`, every field `unverified`, prior record untouched |
| Producer revision unknown | attributed as binary-name@`unknown`, mirroring the deploy-executor precedent; never a claimed version |
| Run revision ≠ captured revision | every field `unverified`; freshness `stale`; a release check citing it is refused |
| Unknown field name | refused by name, counted `refused`, rest of the document preserved |
| `blocked` in an evidence state | refused as vocabulary misuse |
| `verified` without a resolving `evidence_ref` | refused; not downgraded silently, because silent downgrade would hide the producer's error |
| `verified` publication with no digest | refused as contradictory |
| Unparseable or oversized document | `unavailable` quoting the runtime's own bounded words; exit non-zero; prior evidence byte-identical |
| Declaration edited by a user | ownership-conflict refusal, both files byte-preserved |
| Export claims `deployable: true`-worthy completeness | ignored as an authority Forge does not grant; `deployable` is not a field this package ever writes |

## Compatibility and migration

- Additive throughout: no existing document, discriminator, exit code, journal
  kind or declaration template changes.
- `.project.json` `release_evidence` is optional in WG's schema with
  `additionalProperties` typed refs, so an older sibling audit ignores it; the
  audit's own behaviour for a present-but-incomplete block is unchanged, and
  `deployable: false` keeps `validate_deployment_evidence` off the path.
- `forge gate status` output stays byte-identical; the evidence view is a new
  subcommand, so no existing consumer re-parses.
- MCP, API and portal gain no new write surface, and the mature registry
  advertises nothing new — the same boundary `gate-runtime-evidence` held.

## Verification

- Verbatim captured exports from the sibling once it ships, stored under
  `tests/fixtures/gate-evidence/` with a `NOTES.md` recording the producer
  revision and every divergence from the design's expectations — the established
  pattern, which has caught two wrong assumptions before (`gate --dry-run` has no
  JSON composition; the workspace-governance adapter is not executable).
- Classification matrix tests for every row in the failure-boundary table.
- Real round trip: run the gate for real in a scratch project with an initialized
  store, export, consume, confirm `verified` appears only for fields the run
  established, then commit and confirm the same record reads `stale` and the
  release check refuses it.
- This repository: no `verified` field is claimed until
  `driftwatch init` has run here. The evidence recorded in HANDOFF is the honest
  all-`unverified` state plus the named sibling-owned next action.
- Portfolio oracle: `python3 scripts/workspace_check.py --project forge --root ..`
  stays at zero errors, and the new `forge doctor` finding is shown not to change
  a verdict for a project that never consumed an export.
