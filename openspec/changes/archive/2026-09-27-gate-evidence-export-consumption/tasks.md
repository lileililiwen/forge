# Tasks: Consume Driftwatchdog's exported gate evidence instead of asserting evidence status

## 0. Companion gate

- [ ] Confirm the sibling shipped its side before touching Forge:
  `driftwatchdog/openspec/changes/gate-evidence-export/` is archived and the
  export entry point exists in that build. Until then this package stays a
  proposal and no task below is eligible.
- [ ] Capture at least one real export document from the sibling and store it
  verbatim under `tests/fixtures/gate-evidence/` with a `NOTES.md` naming the
  producer revision and every divergence from this design's expectations. Do not
  proceed on the design's guessed shape: the last two guesses about sibling
  behaviour were wrong (the gate dry-run JSON composition, the adapter execute
  bit).

## 1. BFS — Baseline and impact coverage

- [ ] Re-read the three upstreams this package must obey and record their exact
  field and state sets: Workspace Governance `docs/capabilities.md` (nine fields,
  `declared`/`configured`/`verified`/`unverified`, `blocked` reserved for
  capabilities), `platform-contracts` `platform.release-evidence` (required
  `release_id`, `service`, `version`, `released_at`, `evidence_kind`), and Forge's
  own `gate-runtime-evidence` spec (resolution, revision binding, journaling,
  honest projection).
- [ ] Measure the current portfolio claim state so the problem stays concrete,
  re-measuring on this host rather than trusting these numbers: 77 declarations,
  all with a `verification.command`; `evidence_status` split 41 `implemented`,
  33 `verified`, 1 `verified-reference-only`, 1 `mvp-validated`, 1 prose
  paragraph; only 8 declarations carry a `release_evidence` block (the seven
  `deployable: true` projects plus Workspace Governance), so 32 of the 34
  `verified` assertions carry no release-evidence structure; and across the 51
  recorded fields in those 8 blocks the states are 43 `configured`, 3 `declared`
  and 5 `unverified` — **zero `verified`**. The gap is an assertion-without-
  evidence pattern, not a missing-file pattern.
- [ ] Map every Forge surface that will read the consumed record:
  `forge gate status`, doctor findings, the release `gate` check kind, `forge
  check` alert projection, the portal deployments/specs sections, and the
  operations journal's existing `gate` kind.
- [ ] Define `ReleaseEvidenceView` and the refusal reasons, and put the
  failure-boundary table in front of a reviewer before any parser exists.

## 2. DFS — Requirement-by-requirement implementation

- [ ] Implement inbound parsing with the standing boundary rules: bounded read,
  single-document parse, shape discrimination, contract/major gate naming the
  observed version, and unparseable/oversize classified `unavailable` with the
  producer's own bounded words quoted.
- [ ] Implement the vocabulary gate: reject unknown field names by name, reject
  states outside the four release states (including `blocked`), and count refused
  entries rather than dropping them silently.
- [ ] Implement the revision gate (run revision must equal the captured revision;
  mismatch makes every field `unverified`) and the attribution gate (a `verified`
  field requires a resolving `evidence_ref` and a named producer; an unknown
  producer revision attributes as name@`unknown`).
- [ ] Implement the contradiction rule: a `verified` `publication` with no artifact
  digest is refused, and no refusal path may downgrade or clear an existing good
  record.
- [ ] Persist the consumed record atomically beside the gate evidence under
  `.forge/gate/<project-id>/`, revision-bound and producer-attributed; keep the
  existing `gate` journal kind with no new kind; ensure reads journal nothing and
  rehearsals persist nothing.
- [ ] Implement `forge gate evidence [TARGET]` (human and JSON) rendering
  freshness `fresh|stale|absent`, the per-field states with attribution, the
  counts and the refusal reasons, with exit 0 only for a fresh record.
- [ ] Implement the doctor `release-evidence` finding (`pass`/`warn`/`fail`/
  `unverified`) including the specific inconsistency — a declaration asserting
  `verified` while carrying no `release_evidence` block — and keep it
  `applicable: false` outside its scope so it never gates health, maturity or an
  exit code.
- [ ] Project the finding through `forge check` with the existing severity mapping
  and `doctor/<id>` symbols, changing no field of the emitted document schema.
- [ ] Wire the release plane's `gate` check kind to resolve through the consumed
  record at the captured revision, with absent/stale/refused yielding
  `unavailable` and a not-ready plan.
- [ ] Populate `forge/.project.json` `release_evidence` only from a real executed
  run, with `deployable` staying `false` and the non-goal recorded. On this
  checkout, where `.driftwatch` is uninitialized, the honest result is every
  field `unverified` and no `verified` claim; the sibling-owned `driftwatch init`
  next action stays in HANDOFF until someone runs it.
- [ ] Keep generated declarations free of any `release_evidence` block and prove
  the `workspace-metadata-emission` per-profile digests are unchanged.

## 3. BFS — Cross-surface regression and completeness

- [ ] Prove no path can make consumed evidence set `deployable: true`, appear as
  a `forge fleet` healthy state, become an MCP tool, API route or portal control.
- [ ] Re-verify `forge gate status` output byte-identity and that the two views
  cannot disagree about the same run.
- [ ] Re-verify journal independence: consumed-evidence writes add no new kind and
  leave other planes' rows byte-preserved.
- [ ] Re-verify redaction and host-path scrubbing end to end: stdout, the persisted
  record and the journal detail.
- [ ] Re-run `python3 scripts/workspace_check.py --project forge --root ..` and
  confirm zero errors with the new block present, and confirm the portfolio's
  `RELEASE_EVIDENCE_INSUFFICIENT` behaviour for a non-deployable project is
  untouched.
- [ ] Update `docs/provider-evidence.md` (what the consumption establishes and
  what stays `not-run`), `docs/release-readiness.md`, `docs/architecture.md`
  (the evidence boundary), and README's shared-gate section.

## 4. Verification

- [ ] `cargo fmt --all -- --check`; `cargo build`; `cargo clippy --all-targets --
  -D warnings`; `cargo test --all-targets` with the native scaffold test excluded,
  then run separately and recorded.
- [ ] Real round trip with the installed sibling in a scratch project: run the
  gate, export, consume, show `verified` appearing only for fields the run
  established, commit to move the revision, then show the same record reading
  `stale` and the release check refusing it. Record exit codes and verbatim
  output.
- [ ] For this repository, record the honest state (no executed gate pass until
  `driftwatch init` runs here) rather than a claim.
- [ ] `node scripts/check-openspec-change-names.mjs`; `node
  scripts/check-spec-governance.mjs`; `openspec validate --all --strict
  --no-interactive`; `git diff --check` plus review of added files; archive
  without `--skip-specs` only when every scoped scenario has current evidence.
- [ ] Record in `HANDOFF.md` the producer revision consumed, the fixture set, the
  per-field states for Forge itself, and the companion work still open in
  Driftwatchdog and Workspace Governance.
