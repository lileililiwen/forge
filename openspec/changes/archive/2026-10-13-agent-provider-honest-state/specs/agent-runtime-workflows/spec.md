# agent-runtime-workflows Specification (delta)

## ADDED Requirements

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