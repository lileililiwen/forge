# agent-runtime-workflows Specification

## Purpose
Managed agent sessions over the existing PTY agent manager: start, pause, takeover, resume, restart and new-session operations plus spec execution, test, commit and confirm-gated push through shared Core contracts, without replacing the agent runtime.
## Requirements
### Requirement: Managed agent sessions

Forge SHALL expose start, pause, takeover, resume, restart and new-session operations for supported OpenCode/Codex adapters, and route spec run to the selected project and active spec.

#### Scenario: Managed agent sessions success

- **WHEN** a supported spec execution starts and resumes
- **THEN** the session identity, provider and project association remain observable

#### Scenario: Managed agent sessions failure

- **WHEN** the manager disappears or the requested transition is unsupported
- **THEN** Forge reports failed, disconnected or unsupported with recovery guidance

#### Scenario: Managed agent sessions boundary

- **WHEN** a session ends without verification evidence
- **THEN** agent completion remains distinct from verified spec completion

### Requirement: Verified test and Git operations

Forge SHALL expose test, commit and push as separate scoped operations, requiring explicit execution intent for remote writes and preserving unrelated work.

#### Scenario: Verified test and Git operations success

- **WHEN** verified related changes are selected for commit
- **THEN** only their reviewed paths are committed with the originating operation recorded

#### Scenario: Verified test and Git operations failure

- **WHEN** tests fail or requested paths include unrelated edits
- **THEN** completion or commit preparation reports the blocker without silently including those edits

#### Scenario: Verified test and Git operations boundary

- **WHEN** tests pass but push was not requested
- **THEN** Forge records verification without updating any remote

### Requirement: Supervised runtime delegation

Forge SHALL delegate agent session lifecycle to the workspace's existing
supervised runtimes (an Ariadex-class session manager, a Sisyphusfy-class
iteration supervisor) through ordered binary resolution and bounded,
argument-array invocation, and SHALL attribute every reported state to the
backing runtime rather than synthesizing it.

#### Scenario: Session starts on the backing runtime

- **WHEN** an operator starts an agent session selecting the supervised
  provider on a host with the runtime installed
- **THEN** the session record names the runtime, its handle and the
  transition journal shows the delegated verb

#### Scenario: Runtime absent

- **WHEN** the provider binary resolves to nothing on the host
- **THEN** the command reports unavailable listing the resolution attempts
  and bundled providers remain fully usable

### Requirement: Truthful transition mapping

Forge SHALL expose only transitions the backing runtime genuinely supports;
unsupported requests SHALL report the operator's real control path (attach
guidance or the runtime's own pause primitive) and SHALL never claim a
state the runtime did not report.

#### Scenario: Unknown session handle

- **WHEN** the runtime reports no session for the stored handle
- **THEN** Forge surfaces disconnected/lost state, preserves the session
  file and transitions, and does not fabricate an active session

#### Scenario: Malformed runtime outcome

- **WHEN** a delegated run finishes but its outcome document cannot be
  parsed
- **THEN** the result is unverified, never done, and the raw failure is
  bounded and redacted

### Requirement: Independent verification for spec execution

When spec execution is delegated to an iteration supervisor, Forge SHALL
base its run-spec verdict on the supervisor's independent verification
outcome, not on agent completion claims.

#### Scenario: Verified loop

- **WHEN** the supervisor exits clean with verification passed
- **THEN** the journaled run-spec verdict is done with the supervisor named
  as the evidence source

#### Scenario: Retryable failure

- **WHEN** the supervisor reports a retryable or blocked iteration
- **THEN** the verdict is partial with the supervisor's named reason and the
  spec binding intact

### Requirement: No unobserved activity claim

Forge SHALL report a session state of `active` only when a real backing
process was started for that session or a live runtime status confirmed it.
It SHALL NOT synthesize, infer or carry forward an `active` claim from a
recorded prior state, from the presence of a provider binary on `PATH`, or
from any other signal that is not an observation of the session itself. A
provider whose backing runtime this build cannot start SHALL report
`unsupported` with the reason and recovery guidance naming the provider that
does expose a backing runtime, while every requested operation SHALL remain
exposed and answerable for that provider. This constraint SHALL NOT be
relaxed by a flag, an environment variable or an opt-in, and it SHALL NOT
alter how a supervised provider's state is derived.

#### Scenario: Bundled provider with an installed binary

- **WHEN** an operator requests `start`, `resume`, `restart` or
  `new_session` on a session whose provider has no backing runtime adapter in
  this build and the provider binary is present on `PATH`
- **THEN** the recorded state is `unsupported`, never `active`; the evidence
  records the binary probe result and states that Forge started no process for
  the request; and the transition journal keeps the requested verb

#### Scenario: Bundled provider without an installed binary

- **WHEN** an operator requests a session operation on a provider whose binary
  is absent from the host
- **THEN** the command reports unavailable, and the reason states both that
  the binary is absent and that this build exposes no adapter for the
  provider, so no state is recorded and no availability is claimed

#### Scenario: A restart never manufactures a fresh active state

- **WHEN** an operator requests `restart` on a non-terminal session of a
  provider with no backing runtime adapter
- **THEN** the recorded state is `unsupported`, the prior transition log and
  the session's spec binding are preserved, and a terminal session still
  refuses restart

#### Scenario: Operations remain exposed

- **WHEN** an operator inspects the command surface for a provider that
  cannot start a backing runtime
- **THEN** `start`, `pause`, `takeover`, `resume`, `restart` and
  `new-session` are all still accepted and each answers with an honest state,
  evidence and next step

#### Scenario: Supervised provider is unaffected

- **WHEN** an operator requests any session operation on a provider whose
  lifecycle is delegated to a real supervised runtime
- **THEN** the verb is delegated as before, the recorded state is the state
  that runtime's live status reported, the session record names the runtime,
  its handle and the adapter version, and no bundled-provider rule changes
  this path
