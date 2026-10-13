# Proposal: Forge never reports agent activity it did not produce

## Why

`start_or_resume` (`src/agent/transitions.rs:181-209`) answers the bundled
`opencode`/`codex` path with `SessionState::Active` after exactly one
observation: `command_on_path(binary)`. It spawns nothing. No subprocess, no
prompt, no output, no exit code. Its own note says so:

> "`{}` recorded for session `{}`; provider `{}` is present and **the recorded
> session is the source of truth**"

`restart` (`src/agent/transitions.rs:211-237`) is the same shape: a state
pre-check, then `SessionState::Active`, from no observation at all.

A session can therefore report `active` having never run. That is a fabricated
result, and it directly contradicts the discipline Forge enforces on its own
Gate plane:

- `src/gate/model.rs:184-185` — "One per-check row. Unknown or unexpected
  state strings become `unresolved` — never `pass`."
- `src/agent/transitions.rs:509-510` (`map_ariadex_status`) — "A state the
  runtime did not report is never synthesized."

The Ariadex path already holds the line. `invoke_runtime` really spawns;
`live_runtime_status` (:552) already returns `None` for
`!provider.is_supervised()`; `unsupported()` (:239) is the existing honest
mechanism and is already used for `pause`/`takeover` on the bundled path. The
invariant exists in the file. It is simply not applied to `start`, `resume`,
`new_session` and `restart`.

## What Changes

- **`start`/`resume`/`new_session` on a bundled provider report
  `unsupported`, not `active`.** The PATH probe survives as *evidence*
  (`binary: <name> (present on PATH)` / `(absent from PATH)`), and the
  recorded detail names the actual reason: this build has no runtime adapter
  for `opencode`/`codex`, so no process was started. The state flows through
  the existing `unsupported()` helper, so the note/next-step shape is the
  same one `pause`/`takeover` already produce.
- **`restart` on a bundled provider reports `unsupported`, not `active`**,
  through the same helper. The non-terminal pre-check is kept and widened to
  the states a bundled session can actually hold, so `start` → `restart` is
  not a dead end.
- **The missing-binary refusal stays a typed `agent-unavailable`**, but its
  reason stops being a false lead ("install the codex provider" would never
  make this build able to run one) and states both facts: the binary is
  absent *and* this build has no adapter for it.
- **The six operations stay exposed.** `forge agent start|pause|takeover|
  resume|restart|new-session` are untouched in `AgentCommands`; only the
  reported state changes.
- **One honest note in the read-only admin surface.**
  `src/api/admin/agent_identity.rs:210` tells the browser operator
  "recorded session state is the live surface for this provider; no runtime
  probe exists" — for a provider whose record can never be an observation,
  that is the same fabrication one layer up.
- **Canonical spec delta.** `agent-runtime-workflows` gains a
  "No unobserved activity claim" requirement that states the invariant in the
  spec's own terms, with the bundled-provider scenario and the supervised
  non-regression scenario.

**Nothing about the Ariadex path changes.** `ariadex_transition`,
`invoke_runtime`, `map_ariadex_status`, `live_runtime_status`, `backing`
resolution and the delegation journal are byte-identical; a supervised
session still reports whatever `ariadex status --json` said, and still refuses
rather than guessing.

## BFS Impact Map

- **Requirements:** `agent-runtime-workflows` — "Managed agent sessions"
  (:8) requires the six operations to be *exposed* for **supported**
  OpenCode/Codex adapters; "Truthful transition mapping" (:65-70) already
  requires that Forge "SHALL never claim a state the runtime did not report";
  "Supervised runtime delegation" (:44-50) is scoped to the supervised
  provider. One requirement is **added**, none is modified (see design §2 for
  the quoted resolution).
- **Concepts/vocabulary:** `AgentProvider`, `SessionState`,
  `SessionTransition`, `AgentSession`, `AgentTransitionOutcome`,
  `RuntimeBacking`, `LiveRuntimeStatus` and `ForgeError::AgentUnavailable`
  are **unchanged**. No new variant, no new error code, no contract version
  bump: `agent/0.1.0` keeps its shape and every enum is a subset of the wire
  vocabulary that already existed.
- **Modules:** `src/agent/transitions.rs` (the two returns plus the
  pre-check and the refusal text). `src/agent/sessions.rs` is touched only if
  `run_spec`'s bundled arm needs its outcome note updated — its delegation
  path is untouched. `src/api/admin/agent_identity.rs` (one note string).
  `src/cli/agent.rs` and `src/cli/commands.rs` are **untouched**: the
  operations, their flags, their JSON shape and their exit codes do not move.
- **Contracts/data:** `forge-agent/0.1.0` field set unchanged. Persisted
  session files written by earlier builds keep deserializing (no serde
  change); new sessions record `state: "unsupported"`, a value the enum and
  the wire format already accepted. Legacy records that already say `active`
  are read back unchanged — this change never rewrites a stored record.
- **Callers:** `apply_transition` has six callers in `src/agent/sessions.rs`
  (`cmd_agent` start/new-session, `run_agent_transition` for
  pause/takeover/resume/restart, `run_spec`'s bundled arm) plus the lib tests.
  Every caller renders `outcome.state` generically; the read-only admin
  handler renders the *recorded* state. No caller needs a code change.
- **Persistence:** the session file and `transitions.log` are still written
  (the operation journal still records `done` — the command completed; the
  detail line carries `-> unsupported`). One transition row per requested
  verb, as before. No new row type, no migration.
- **Integrations:** `ariadex` (unchanged), `sisyphusfy` (`run_spec_sisyphusfy`
  really spawns a supervisor and its verdict is independent-verification
  based — unchanged), DriftWatch, OpenPanel, Hypora, Hermora, driftwatchdog
  (untouched). No new subprocess, dependency, env var or network call is
  introduced, and the change *removes* the risk of an operator reading
  `active` as proof a sibling binary was launched.
- **Failures:** every existing refusal is preserved — unknown provider,
  unknown session, invalid session id, `MAX_TRANSITIONS_PER_SESSION`,
  `run_spec` without a bound spec, the missing-spec `spec-invalid`, the
  restart state guard (widened, not narrowed), and the supervised refusals.
  No typed error code changes.
- **Tests:** `src/agent/agent_tests.rs` (two tests encode the bug and are
  rewritten to the honest oracle; new tests pin the matrix), and
  `tests/agent_contract.rs` (two tests encode the bug and are rewritten).
  `tests/supervised_agent_contract/legacy_session.rs` and
  `tests/supervised_agent_cross_surface.rs` must stay green unchanged — they
  are the Ariadex/bundled non-regression proof.
- **Compatibility:** the reduction is deliberate and is called out in the
  design and HANDOFF: an operator can no longer obtain `active` from
  `opencode`/`codex` in this build, because this build starts nothing for
  them. Sessions they already recorded keep their stored state. The
  supervised path is byte-identical.
- **Concerns:** `quality.md` (every affected provider×transition pair has
  observable success/failure/boundary evidence; unknown is not healthy);
  `security.md` (the change *removes* an unverifiable claim about external
  binary execution; no argument array, shell, env var or credential path is
  added).

## Capabilities

- **`agent-runtime-workflows`** (modified): the agent plane holds the same
  never-claim-unobserved-activity invariant the Gate plane already holds —
  no session reports `active` unless a real process was started or a live
  status was confirmed.

## Non-goals

- **Implementing real spawning for `opencode`/`codex`.** That is option (b)
  in design §2 and it is rejected on the quoted spec evidence: it would
  replace the existing PTY manager, which `AGENTS.md` forbids, and no
  canonical spec asks for it.
- **Touching the Ariadex path.** Not one line of `ariadex_transition`,
  `invoke_runtime`, `map_ariadex_status`, `live_runtime_status`,
  `disconnected_outcome` or `RuntimeBacking` changes.
- **Removing the operations or the provider ids.** `start`, `pause`,
  `takeover`, `resume`, `restart` and `new-session` all stay exposed for
  `opencode`/`codex`/`ariadex`; the spec requires the surface to exist.
- **Rewriting stored session files.** No migration, no backfill: a record
  written by an earlier build reads back exactly as written.
- **Changing `AgentProvider`, `SessionState`, `validate_session_id` or
  `resolve_binary_path`.** Explicitly out of scope by the brief.
- **Touching `src/gate/`, `src/contract/` or `src/api/command_catalog/`.**
  The Gate plane is the reference behaviour, not the subject.
- **A second durable plan.** `proposal.md`, `design.md`, `tasks.md` and the
  canonical `specs/` delta are the only plan of record.
- **Phase:** closing an honesty defect. No publish, no push, no release.