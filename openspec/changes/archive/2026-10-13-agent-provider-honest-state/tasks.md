# Tasks: Forge never reports agent activity it did not produce

## Phase 1 — BFS baseline

- [x] 1.1 `node scripts/check-openspec-change-names.mjs` PASS before
      selection; `openspec list` reports no active change.
- [x] 1.2 Read `AGENTS.md`, `README.md`, `ROADMAP.md`, `HANDOFF.md`,
      `.ai-rules/workflow.md`, `.ai-rules/completion.md`,
      `.ai-rules/concerns/quality.md`, `.ai-rules/concerns/security.md`.
- [x] 1.3 Read the canonical agent specs before changing agent code:
      `openspec/specs/agent-runtime-workflows/spec.md` (all five
      requirements) and
      `openspec/specs/supervised-agent-contract-size-split/spec.md`.
- [x] 1.4 Establish which canonical specs mention OpenCode/Codex at all:
      `rg -l -i 'opencode|codex' openspec/specs/ --glob '*.md'` returns
      exactly one file and, inside it, exactly one line
      (`agent-runtime-workflows/spec.md:8`). No README/HANDOFF/ROADMAP claim
      requires them to run; `requirement.md:900-905` says "Supported agents
      **may include**".
- [x] 1.5 Read the defect path end to end: `src/agent/transitions.rs`
      (`apply_transition`, `start_or_resume`, `restart`, `unsupported`,
      `next_step_for`, `invoke_runtime`, `map_ariadex_status`,
      `live_runtime_status`, `ariadex_transition`), `src/agent/model.rs`
      (the whole vocabulary), `src/agent/sessions.rs`
      (`command_on_path`, `probe_provider`, `run_spec`,
      `run_spec_sisyphusfy`), `src/cli/agent.rs`, `src/cli/commands.rs`,
      `src/api/admin/agent_identity.rs`.
- [x] 1.6 Confirm the Ariadex path is genuinely real and is the reference:
      `invoke_runtime` calls `Command::spawn` (`transitions.rs:387`) with an
      argument array, null stdin, project cwd and a bounded wait;
      `map_ariadex_status` only maps a live `alive` daemon to `Active`;
      `live_runtime_status` already returns `None` for
      `!is_supervised()`.
- [x] 1.7 Map every caller of `apply_transition` (6 in
      `src/agent/sessions.rs`) and every consumer of `outcome.state`
      (`src/cli/agent.rs` renders it generically in both formats;
      `agent_identity.rs` renders the *recorded* state). No caller needs a
      code change.
- [x] 1.8 Find every existing test that encodes the false-`active` behaviour
      and decide each: `src/agent/agent_tests.rs` (2 tests assert
      `SessionState::Active` outright), `tests/agent_contract.rs` (2 tests —
      one asserts `active` outright, one accepts `active || unsupported`).
      Also read `tests/agent_runtime_workflows_cross_surface.rs`,
      `tests/supervised_agent_contract/*` and
      `tests/web_agent_identity_readonly_contract.rs` to confirm which are
      already honest and must stay untouched.
- [x] 1.9 Record the (a)/(b) decision and its quoted evidence before writing
      any code: design §2. Option (b) is rejected on three grounds — no
      canonical scenario requires a bundled provider to start a process, and
      `AGENTS.md:50` forbids replacing the existing PTY manager, which is not
      wired into this build in any case.
- [x] 1.10 Record the before/after matrix per provider × transition in design
      §4, including the full Ariadex matrix, so the non-regression claim is
      checkable rather than asserted.
- [x] 1.11 Set the single `current_spec: agent-provider-honest-state` pointer
      in HANDOFF before implementation.

## Phase 2 — DFS requirement implementation

- [x] 2.1 `start_or_resume`: keep the `command_on_path` probe as evidence,
      route the return through the existing `unsupported()` helper, and put
      the probe result plus the "no adapter was spawned" reason into the
      detail line. No new state, no new error, no new dependency.
- [x] 2.2 `restart`: keep the non-terminal pre-check, widen it to the five
      non-terminal states (a bundled session's steady state is now
      `Unsupported`, so the old three-state guard would make `start` →
      `restart` a dead end), correct its refusal text to name them, and route
      the successful case through `unsupported()`.
- [x] 2.3 Correct the missing-binary refusal text so it stops telling the
      operator to install a provider that would not help; keep the typed
      `agent-unavailable` code and exit status unchanged.
- [x] 2.4 Reword the one read-only admin note
      (`src/api/admin/agent_identity.rs:210`) so it no longer calls the
      recorded state "the live surface" for a provider whose record is never
      an observation. Keep the `recorded` substring the pinned test asserts.
- [x] 2.5 Confirm by diff that nothing under `src/gate/`, `src/contract/`,
      `src/api/command_catalog/`, `src/agent/model.rs` or `src/cli/` changed,
      and that no Ariadex function body changed.

## Phase 3 — BFS regression and completeness

- [x] 3.1 Rewrite `agent_tests::start_records_state_for_present_or_missing_provider`
      — its `assert_eq!(outcome.state, SessionState::Active)` encoded the
      bug — to assert `unsupported` for the present-binary arm and
      `agent-unavailable` for the absent-binary arm.
- [x] 3.2 Rewrite `agent_tests::restart_preserves_prior_state_and_records_active`
      — same defect — as
      `restart_preserves_the_prior_record_without_claiming_active`, asserting
      `unsupported`, one recorded transition and the preserved spec binding.
- [x] 3.3 Rewrite `agent_contract::start_records_session_with_provider_and_project_association`
      — `s == "active" || s == "unsupported"` accepted the fabrication — so
      the state is `unsupported` or a typed refusal, never `active`, while
      the session id, project id, provider and both written files stay
      asserted.
- [x] 3.4 Rewrite `agent_contract::restart_preserves_prior_state_and_records_active`
      — it asserted `active` outright — to assert `unsupported` with the
      session id and spec binding preserved.
- [x] 3.5 Add `start_on_a_bundled_provider_never_claims_active`: start,
      new_session and resume on both bundled providers are `unsupported`, the
      evidence names the probe result and the no-adapter reason, the note
      says the session stays `unsupported`, and a next step is offered.
- [x] 3.6 Add `restart_on_a_bundled_provider_never_claims_active`: every
      non-terminal prior state returns `unsupported`, a `Stopped` session
      still refuses with `agent-unavailable`, and the spec binding survives.
- [x] 3.7 Add `no_transition_reaches_active_without_a_process_or_a_live_report`:
      the exhaustive `is_supervised()` × `SessionTransition` matrix, asserting
      that no bundled provider × any transition can yield `Active`, and that
      the operations all remain reachable (i.e. they answer, not vanish).
- [x] 3.8 Run the unchanged Ariadex/sisyphusfy non-regression suites and
      record their counts as evidence that the supervised path is
      byte-identical: `tests/supervised_agent_contract`,
      `tests/supervised_agent_cross_surface`.
- [x] 3.9 Run `tests/agent_runtime_workflows_cross_surface` and
      `tests/web_agent_identity_readonly_contract` unchanged.
- [x] 3.10 `git diff --check` clean, including newly added files; the
      pre-existing rustfmt drift in `tests/web_assurance_browser_contract.rs`
      and `src/gate/rehearsal_tests.rs` is untouched and unstaged.

## Phase 4 — Verification

- [x] 4.1 `cargo build`.
- [x] 4.2 `cargo fmt --check` clean for every touched file.
- [x] 4.3 `cargo clippy` (default targets) — no new finding.
- [x] 4.4 `cargo test --lib agent` and `cargo test --lib gate`.
- [x] 4.5 `cargo test --test agent_contract`.
- [x] 4.6 `node scripts/check-openspec-change-names.mjs` PASS.
- [x] 4.7 `node scripts/check-spec-governance.mjs` PASS.
- [x] 4.8 `openspec validate --all --strict --no-interactive` PASS.
- [x] 4.9 `forge gate --dry-run` rehearsed; the full `forge gate` is the
      orchestrator's step for this cycle and is recorded as delegated, never
      as a pass.
- [x] 4.10 Archive `agent-provider-honest-state` **without** `--skip-specs`;
      commit 1 = implementation + tests + promoted specs + archive, explicit
      paths only; commit 2 = HANDOFF evidence and pointer removal. No push.
- [x] 4.11 Re-run `openspec validate --all --strict --no-interactive` after
      the archive.