# agent-runtime-workflows Specification

## ADDED Requirements

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
