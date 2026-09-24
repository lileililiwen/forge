# Design: Supervised runtime adapters

## Ownership and boundaries

Ariadex owns tmux sessions, provider lifecycle and handoff prompts.
Sisyphusfy owns iteration loops, model fallback and independent
verification. Forge owns project identity, the session record that points
at them, and the normalized transition/outcome vocabulary. Forge never
synthesizes runtime state; every reported transition is what the backing
tool said, attributed by name and version probe.

## Adapter shape

Same executable-adapter discipline as the deploy/analytics planes: ordered
binary resolution (env override → PATH name), argument arrays only, bounded
timeouts, redacted captured output, explicit `unavailable` on spawn/exit/
timeout/parse failure.

### Ariadex mapping

- `forge agent start --provider ariadex --path P` → `ariadex start` in P
  (the sibling's documented ensure-daemon entry); Forge records the session
  with `backing = {runtime: "ariadex", handle: <project/session id as the
  sibling reports>}`.
- `status` → parse the sibling's documented status surface into
  SessionState (`active|paused|disconnected|…`); unknown states map to
  `disconnected` with raw state preserved in evidence, never to `active`.
- `resume` → `ariadex resume` (idempotent from its PAUSE state per the
  sibling's docs). `pause` → the sibling's real pause primitive if exposed;
  otherwise report `unsupported` **naming the operator path** (attach/
  widget control), replacing today's generic PTY note.
- `takeover` → pointer output: the exact `ariadex` command an operator runs
  to attach, journaled as guidance, no process hijacking.
- `restart`/`new-session` → stop-then-start through the sibling's verbs.

### Sisyphusfy mapping (spec execution)

- `forge agent run-spec SPEC --provider sisyphusfy` writes nothing new:
  the existing `.forge/specs/<spec>/` proposal/design/tasks files are the
  task input the sibling already consumes; Forge invokes the loop with its
  documented flags and `--json`.
- The outcome document maps to the run-spec evidence block: verification
  pass + clean exit → `done`; retryable-failure classification → `partial`
  with the sibling's named reason; unparsable outcome → `unverified`.
  Agent self-claims never upgrade to done (the sibling's verification
  verdict is what counts, matching Forge's own evidence rule).

## Session record

`AgentSession` gains `backing: Option<RuntimeBacking {runtime, handle,
adapter_version}>`. Bundled adapters keep `backing: None` and current
behavior; old session files deserialize unchanged (serde default).

## Failure taxonomy

- spawn/PATH failure → `agent-unavailable` (existing code) with the
  resolution list.
- runtime says "no such session" → `agent-unavailable` scoped to the
  handle; session file and transitions preserved; inspect shows last known
  state plus the loss.
- timeout → bounded wait exceeded note; state `disconnected`.
- version probe mismatch against the sibling's documented surface →
  `unsupported` with the version named rather than best-effort parsing.

## Verification

- Fixtures: stub `ariadex`/`sisyphusfy` scripts emitting the documented
  shapes (success, paused, unknown-session, malformed JSON).
- Contract: transition matrices per provider; cross-surface check that the
  run-spec evidence flows through the spec-remediation contract unchanged.
- Live note: on hosts with the real binaries, one documented round trip per
  provider recorded as evidence; otherwise rows stay `not-run`.
