# Tasks: consume the sibling's gate plan rehearsal document

## Phase 1 — BFS baseline

- [x] 1.1 `node scripts/check-openspec-change-names.mjs` PASS before
      selection; `openspec list` reports no active change.
- [x] 1.2 Read `AGENTS.md`, `.ai-rules/workflow.md`,
      `.ai-rules/completion.md`, `.ai-rules/concerns/quality.md`,
      `.ai-rules/concerns/security.md`, and the canonical specs
      `gate-runtime-evidence` and `gate-evidence-export-consumption`.
- [x] 1.3 Reproduce the defect live: with
      `FORGE_GATE_BIN=~/code/driftwatchdog/target/release/driftwatchdog`,
      `forge gate --dry-run` prints the rehearsal JSON indented as plan
      lines; the same command against `~/.cargo/bin/driftwatchdog`
      (Oct 5) prints the human plan. Both binaries report
      `driftwatch 0.1.0`.
- [x] 1.4 Read the producer's authoritative struct
      (`driftwatchdog/src/gate/dry_run_plan.rs`, read-only) and record the
      two invariants it guarantees: no verdict-shaped key at any depth,
      and `would_block` is `true` or `null`, never `false`.
- [x] 1.5 Map every affected surface: `src/gate/engine.rs`
      (`run_gate`, `journal_verdict`), `src/gate/model.rs`
      (`GateOutcome`), `src/gate/mod.rs` (module docs),
      `src/cli/functions_13.rs` (`cmd_gate_run`), the new
      `src/gate/rehearsal.rs`, and the canonical spec
      `openspec/specs/gate-runtime-evidence/spec.md`.
- [x] 1.6 Audit every consumer of `GateOutcome` and of
      `GateOutcome::PlanPreview` (`rg 'GateOutcome::|PlanPreview'`): the
      only consumers are `run_gate` (constructs),
      `journal_verdict`, and `cmd_gate_run` (renders). `doctor`,
      release prepare, readiness and `forge contract emit` read
      *persisted* evidence through `load_latest_evidence` only.
- [x] 1.7 Capture the live rehearsal document verbatim as
      `tests/fixtures/gate/gate-rehearsal-plan.json` and record its
      provenance in `tests/fixtures/gate/NOTES.md`, keeping the older
      human-text capture as the stale-binary shape.
- [x] 1.8 Set `current_spec: gate-rehearsal-document-consumption` in
      HANDOFF.

## Phase 2 — DFS requirement implementation

- [x] 2.1 `src/gate/rehearsal.rs`: wire serde types for the rehearsal
      root, `planned_checks[]`, `blocking_prediction` (+ reasons),
      `exemption_budget`, `blocking_policy` and `source_size_policy`,
      and the sanitized display struct. No new dependency.
- [x] 2.2 Implement the shape-detection rule exactly as design §3b: the
      `driftwatch-gate-plan/` contract family, `document_kind`,
      `dry_run: true`, `executed: false`, `persisted: false`, a complete
      typed parse, and `would_block != false`. Every captured string
      goes through `capture_note`.
- [x] 2.3 Add `GateOutcome::Rehearsal { runtime, runtime_version,
      rehearsal }` to `src/gate/model.rs` and correct the variant
      documentation that claims a rehearsal can arrive as evidence.
- [x] 2.4 `run_gate`: run the detector **before** `parse_gate_document`;
      return `Rehearsal` for a dry run and the new fail-closed
      `Unavailable` reason when a rehearsal answers a non-dry-run
      invocation. The degrade branch is unchanged.
- [x] 2.5 `journal_verdict`: the preview and rehearsal arms return
      `"rehearsed"`, never `"done"`. `Evidence` and `Unavailable` arms
      untouched.
- [x] 2.6 `render_rehearsal_human` and `rehearsal_envelope`: planned
      checks with required/optional, command source, `selected_by` and
      timeout; blocking policy; default timeout; source-size max lines;
      manifest digest; rule pack; profile; plan version; exemption
      budget; unbound required checks; not-scheduled; and the blocking
      prediction with `would_block=unknown` wording. Rows bounded by
      `MAX_PLAN_LINES` with an explicit remainder line.
- [x] 2.7 Mirror the closed verdict-shaped deny-set and its
      any-depth walker so the emitted envelope is provably verdict-free.
- [x] 2.8 `src/cli/functions_13.rs`: the `Rehearsal` render arm —
      envelope with `persisted: false` / `journaled: false`, `plan`
      retained alongside `rehearsal`, human render, exit success. No
      `save_evidence`, no `record_operation`.
- [x] 2.9 Correct the two stale comments: `src/gate/mod.rs` (the dry-run
      surface now has a JSON composition) and `src/gate/model.rs` (the
      fallback is not "the sibling has no JSON"; it is the
      unrecognised-shape degrade).
- [x] 2.10 Register the module and its test submodule in `src/gate/mod.rs`
      and `src/gate/rehearsal.rs`.

## Phase 3 — BFS regression/completeness

- [x] 3.1 `src/gate/rehearsal_tests.rs`: the verbatim capture parses into
      the typed document — 9 planned checks, all required,
      `source-file-size` `in-process`, the rest `manifest`, every
      timeout `60000`, digest, rule pack, profile, policy, exemption
      budget, empty unbound list, prediction `undetermined-before-execution`
      with `would_block == None`.
- [x] 3.2 Render test: every planned check id, `[required]`,
      `command=in-process`, `command=manifest`, timeouts,
      `review_required_blocks=true`, the digest, the rule pack and the
      `would_block=unknown (the bound commands decide)` wording, plus the
      "never an observed outcome" statement.
- [x] 3.3 Detection table: one row per rule — missing `contract`, wrong
      contract family, missing `document_kind`, wrong `document_kind`,
      `dry_run: false`, `executed: true`, `persisted: true`,
      `would_block: false`, renamed required field, truncated array — each
      must return `None` and degrade.
- [x] 3.4 Never-evidence proofs: `run_gate` over the capture returns
      `Rehearsal` (not `Evidence`), creates no evidence file, and
      `journal_verdict` is `rehearsed`; `save_evidence` still refuses a
      `dry_run` record.
- [x] 3.5 Deny-set proof over the real emitted envelope: no
      `status`/`result`/`results`/`passed`/`blocked`/`outcome`/
      `aggregate`/`exit_code`/`gate_status`/`ok`/`success`/`failures`/
      `pending_reviews` key at any depth.
- [x] 3.6 Degrade tests: the human-text capture still yields
      `PlanPreview` with bounded plan lines; an unrecognised rehearsal
      shape does the same; neither is `Evidence` and neither is a pass.
- [x] 3.7 Real-document parity: each of the three verbatim status
      captures still maps through `parse_gate_document` to the same
      aggregate and per-check rows, and none is detected as a rehearsal;
      the rehearsal capture is not a status document either
      (`parse_gate_document` returns `None`).
- [x] 3.8 Contradiction refusal: a recognised rehearsal returned to a
      non-dry-run invocation is `Unavailable` naming the contradiction,
      never evidence and never a pass.
- [x] 3.9 `tests/gate_contract/documented_help.rs`: the end-to-end
      rehearsal case — rendered envelope with `planned_checks`, no
      `.forge` tree in the project, no journal row.
- [x] 3.10 Confirm the canonical spec scenarios are updated and that
      `gate-evidence-export-consumption` and `src/gate/evidence.rs` are
      untouched.

## Phase 4 — Verification

- [x] 4.1 `cargo build`; `cargo fmt --check`; `cargo clippy` (default
      targets).
- [x] 4.2 `cargo test --lib gate::`; `cargo test --lib`.
- [x] 4.3 `node scripts/check-openspec-change-names.mjs` PASS.
- [x] 4.4 `node scripts/check-spec-governance.mjs` PASS.
- [x] 4.5 `scripts/contract-parity.sh` exit 0 with the compared counts.
- [x] 4.6 `openspec validate --all --strict --no-interactive` PASS.
- [x] 4.7 `git diff --check` clean; reviewed the full staged set
      including newly added files.
- [x] 4.8 `forge gate --dry-run` against the current sibling build
      renders the typed rehearsal (a rehearsal executes nothing); paste
      the output into the HANDOFF evidence.
- [x] 4.9 Archive `gate-rehearsal-document-consumption` **without**
      `--skip-specs`; commit 1 = implementation + tests + specs + archive,
      explicit paths only; commit 2 = HANDOFF evidence and pointer
      removal. No push.
- [x] 4.10 The heavy Gate runtime (bounded `forge gate`) is the
      orchestrator's step for this cycle; the HANDOFF evidence table
      records that delegation plus the two pre-existing blockers rather
      than claiming a verdict.
