# Proposal: Consume Driftwatchdog's exported gate evidence instead of asserting evidence status

## Companion gate — authored, not eligible

Driftwatchdog's `gate-evidence-export` change (`driftwatchdog/openspec/changes/gate-evidence-export/`,
workspace package 4) is **authored and unselected** — nothing is implemented
there yet. Its own tasks record "Record the governance-side consumption as the
next action; do not claim it here", and its consumer list already names Forge
("Consumers: Workspace Governance (audit), **Forge**, Jenkins-local, and project
release workflows").

This package is therefore the Forge-side half of that pair: authored now,
implemented only after the sibling exports a real document that can be captured.
No task below may run before that companion evidence exists, and nothing here
defines the export's shape — Driftwatchdog owns it.

## Why

The portfolio's evidence claims are currently unfalsifiable in one specific way.
Measured on this host across `*/.project.json`:

- 77 declarations exist, all carrying a non-null `verification.command`, and 77
  carry `verification.evidence_status`: 41 `implemented`, 33 `verified`, 1
  `verified-reference-only`, 1 `mvp-validated`, 1 an entire prose paragraph —
  and no declaration on this host still holds the template's `planned`;
- only **8** carry a `release_evidence` block at all (the seven
  `deployable: true` projects plus Workspace Governance), so **32 of the 34
  declarations asserting a `verified` evidence status carry no
  release-evidence structure whatsoever**;
- across those 8 blocks all 51 recorded fields sit at `configured` (43),
  `declared` (3) or `unverified` (5). **Not one field in the portfolio reads
  `verified`**, even though Workspace Governance reserves that state for
  "native evidence recorded with a ref".

Workspace Governance's own rules make the asymmetry a failure mode rather than a
cosmetic gap: a `deployable: true` project must carry a complete
`release_evidence` block or `workspace_check.py` fails it closed with
`RELEASE_EVIDENCE_INSUFFICIENT`, and `jenkins_manifest.py` will not emit the
entry (README, "Capability and release evidence"). So the portfolio asserts
"verified" 34 times while the structure that would substantiate it exists for
eight projects, is hand-written rather than executed, and grants no verified
field to anyone.

Driftwatchdog identified the same thing as the reason for package 4: "the
portfolio's seven deployable projects carry hand-written `configured` states that
point at pilot-context placeholders, and the governance audit cannot distinguish
an executed gate from a declaration. **73 `.ai-gate/gate.yaml` declarations exist
in the workspace and only this repository's own CI runs one.**" Its export maps
only checks that actually ran and passed to `verified`, refuses `verified` when
the run's revision differs from the project's current revision, and consumes
Workspace Governance's field names and state vocabulary rather than re-declaring
them in Rust.

Forge is the consumer that makes that export useful. Today Forge already
*executes* the gate runtime and journals revision-bound evidence
(`gate-runtime-evidence`), and already *writes* declarations that Workspace
Governance reads (`deterministic-project-generation`, the
`workspace-metadata-emission` cycle). What it does not do is connect those two:
it has no inbound path for an executed gate's release evidence, so Forge's own
declaration has no way to move from an asserted state to an evidenced one, and a
governed project has no way to record what its gate actually established. That
is the loop this package closes.

The vocabulary already exists on both sides and must not be re-invented here:
Workspace Governance owns the field names (`revision`, `version`, `toolchain`,
`artifacts`, `digests`, `sbom`, `provenance`, `checks`, `publication`) and the
states (`declared`, `configured`, `verified`, `unverified`, with `blocked`
reserved for capability declarations); `platform-contracts` owns the wire shape
of `platform.release-evidence` (`required: release_id, service, version,
released_at, evidence_kind`, owned runtime "driftwatchdog (release publisher)").
Forge's job is to consume all three faithfully and to be strict about what it
will not claim.

## What Changes

- Add an **inbound consumption boundary** for the export document: parse and
  validate it against the consumed vocabulary and the vendored schema, refuse an
  unknown field name, refuse a state outside the vocabulary, and classify any
  unparseable, oversized or revision-mismatched document as an honest
  `unavailable`/`unverified` outcome that leaves prior evidence untouched.
- Record the consumed export as **attributed evidence**: the producer, its
  revision, the gate run's revision and the observed timestamp, so a reader can
  tell which executed run supports which claim.
- Add a read surface (`forge gate evidence [TARGET]`, human and JSON) that
  reports the consumed release-evidence view of the last executed run, distinct
  from `forge gate status`, which reports the gate verdict.
- Populate this repository's own `release_evidence` block **only** from a real
  executed run: `revision`, `version`, `toolchain` and `checks` in `verified`
  state when the export says so, and `unverified` for `artifacts`, `digests`,
  `sbom`, `provenance` and `publication` for as long as Forge publishes nothing
  (which is the current state and stays recorded as such). This is the first
  declaration in the portfolio whose states are produced by an executed gate
  rather than hand-written.
- Add a doctor finding that reports the release-evidence state distribution for a
  project and flags the specific inconsistency the audit cannot see today: a
  declaration asserting `verified` while carrying no `release_evidence` block.
  It stays non-gating, like the other evidence findings.
- Wire the consumption into the existing gate and release surfaces: the release
  `gate` check kind may cite the consumed evidence at the captured revision, and
  `forge check` projects it as an alert without any schema change.
- Keep `deployable: false` and the deployment defaults exactly as they are.
  Consumed evidence does not make a project deployable; Workspace Governance's
  promotion path does, and its rule that evidence fields "never will promote a
  project to deployable on their own" is honoured here.

## BFS Impact Map

- **Capabilities:** new `gate-evidence-export-consumption`; modifies
  `gate-runtime-evidence` (inbound document consumption alongside execution),
  `release-publishing` (checks may cite consumed evidence),
  `doctor-maturity-assessment` (new non-gating finding),
  `deterministic-project-generation` (the declaration's evidence vocabulary),
  `external-checker-emission` (projection only, schema unchanged),
  `platform-contract-consumption` (the `platform.release-evidence` wire shape and
  vendored schema), and `governance-vocabulary-consumption` (field and state
  names come from consumed vocabulary, never locally declared).
- **Users and flows:** an operator running a gate gets a machine-consumable
  evidence record instead of a terminal verdict; Workspace Governance's audit can
  distinguish an executed gate from an assertion for the first time; a project
  preparing to become deployable sees exactly which evidence fields are still
  missing rather than discovering it at the promotion step.
- **Contracts/data/persistence:** the inbound document is Driftwatchdog's, not
  Forge's, and is treated as untrusted input; consumed evidence persists under
  the gate plane's existing `.forge/gate/<project-id>/` layout, revision-bound,
  atomically written, with the journal keeping the append-only history;
  `.project.json` gains the optional `release_evidence` block for this
  repository only, and no generated project's declaration changes.
- **Integrations/configuration:** Driftwatchdog's export entry point; Workspace
  Governance's `docs/capabilities.md` vocabulary (consumed through
  `governance-vocabulary-consumption`); `platform-contracts`'s
  `platform.release-evidence` schema (consumed through
  `platform-contract-consumption`); no new binary probe, no network, no new
  provider row.
- **Callers:** `forge gate`, the release plane's `gate` check kind, doctor, the
  checker plane, the portal deployments/specs sections, and
  `scripts/check-spec-governance.mjs` if a status claim needs requalifying.
- **Failure/boundary behavior:** absent export → `unverified` with the reason
  named, never a silence that reads as health; revision mismatch → every field
  `unverified`, mirroring the producer's own refusal rule; unknown field name or
  out-of-vocabulary state → refused and classified `unavailable`, prior evidence
  untouched; oversized or unparseable document → `unavailable`; a `verified`
  `publication` claim with no artifact digest → refused as contradictory rather
  than recorded.
- **Tests:** the classification matrix above; revision binding and staleness;
  attribution round trip; atomic persistence with prior-record preservation;
  doctor's non-gating guarantee; the checker alert projection; port
  byte-identity for projects without an export; and a cross-surface test proving
  that consumed evidence cannot make `deployable: true` appear.
- **Dependencies:** hard on Driftwatchdog shipping the export; hard on
  `platform-contract-consumption` (vendored schema and digest mechanism) and
  `governance-vocabulary-consumption` (field and state vocabulary); ordered last
  among this set because it consumes both.
- **Compatibility/security/privacy:** additive; no existing document, exit code
  or journal shape changes; captured strings pass
  `policy::redact_credentials` with the assessed project's absolute path replaced
  by `<project>` and char bounds applied, exactly as the gate plane already does;
  artifact digests are recorded as references, and no secret material ever enters
  a declaration or a journal detail.

## Capabilities

- `gate-evidence-export-consumption`: Forge can consume an executed gate's
  release-evidence export, attribute it to its producer and revision, refuse
  anything its vocabulary or evidence does not support, and surface it honestly
  across doctor, the checker plane and the release checks — so a declaration's
  evidence state can be derived from a real run instead of asserted.

## Non-goals

- Defining the export document. Driftwatchdog owns it, including the export
  entry point, its field semantics and its refusal rules.
- Reaching `deployable: true` for Forge or for any project. Deployment promotion
  belongs to Workspace Governance's promotion path and Jenkins-local; consumed
  evidence never promotes a project by itself.
- Publishing, signing, SBOM generation or provenance production. Forge records
  what a producer asserted and refuses to claim what no producer produced.
- Retro-filling `release_evidence` into other repositories' declarations, or
  into generated project templates. A generated project has run no gate and gets
  no evidence block.
- Substituting Forge-side heuristics for a missing export: no synthesized
  `verified`, no inherited status from a previous revision, no pass by silence.
- Replacing the gate verdict surface. `forge gate status` and `forge doctor` keep
  their current meanings; this adds an evidence view beside them.

Source: requirement.md §24, §25, §29, §32, §34, §39, §43, §45.
