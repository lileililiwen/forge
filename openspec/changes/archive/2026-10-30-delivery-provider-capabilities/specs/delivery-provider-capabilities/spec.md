## ADDED Requirements

### Requirement: The capabilities operation is reachable and recorded

Forge SHALL invoke the publish-provider `capabilities` operation once at the
start of every delivery verb that invokes the publish provider, after the
verb's existing local gates and before the delivery operation is reserved. The probe SHALL carry no `queue_id`, SHALL
reserve no operation and SHALL change no delivery phase. The provider's
declaration SHALL be recorded on the journal row of the verb it gated, as a
bounded `provider_capabilities` value naming either the declared operations or
the reason the provider made no readable claim.

#### Scenario: A provider declares its operations

- **WHEN** a delivery verb resolves a provider and the provider answers the
  `capabilities` operation with a valid list
- **THEN** Forge records the declaration on that verb's journal row and the
  verb proceeds under the rule in the next requirement

#### Scenario: The probe has no side effects

- **WHEN** Forge probes a provider's capabilities
- **THEN** the request carries no `queue_id`, no delivery operation row is
  reserved for the probe, and the delivery phase is unchanged by it

#### Scenario: The probe does not answer

- **WHEN** the provider exits non-zero, returns no readable provider response,
  or returns a response with no `capabilities` field
- **THEN** the declaration is recorded as unknown with the reason, and the verb
  is judged by the unknown rule rather than refused for the probe itself

### Requirement: A declaration gates the delivery verbs

A provider that declares a capability set SHALL be refused for a delivery verb
whose provider operation the set does not contain. The refusal SHALL happen
before the requested provider operation runs and before the delivery operation
is reserved, and it SHALL name the delivery verb, the provider, the required
operation and the declared operations. `preflight` SHALL NOT be gated by a
declaration. A provider that makes no readable claim SHALL be treated as
unknown, which permits `preflight` and `publish` and permits no other
operation.

#### Scenario: A declared provider is served

- **WHEN** the provider declares `preflight` and `publish`
- **THEN** preflight, stage and promote reach the provider

#### Scenario: A declared set omits the required operation

- **WHEN** the provider declares `preflight` but not `publish` and the user
  invokes stage or promote
- **THEN** Forge refuses with a typed delivery error naming the verb, the
  provider, `publish` and the declared operations, writes no delivery row for
  the refused verb, runs no `publish` operation, and leaves the phase at
  `preflighted`

#### Scenario: An unknown provider is served by the existing floor

- **WHEN** the provider makes no readable claim and the user invokes preflight,
  stage or promote
- **THEN** the verb proceeds, because unknown permits `preflight` and `publish`

#### Scenario: An unknown provider is refused a newer operation

- **WHEN** the provider makes no readable claim and a verb requires an
  operation other than `preflight` or `publish`
- **THEN** Forge refuses that verb instead of attempting the operation

#### Scenario: An empty declaration is a declaration

- **WHEN** the provider answers with an empty capability list
- **THEN** the provider declares nothing, and stage and promote are refused
  under the declared rule

### Requirement: A malformed capability claim is refused

A provider response whose `capabilities` field carries more than sixteen names,
a name that is empty, longer than thirty-two characters or outside
`[a-z0-9_-]`, or the same name twice SHALL be refused as an invalid provider
response. A name Forge does not recognise inside an otherwise valid list SHALL
be ignored, so a provider declaring a newer operation is readable by this
Forge. The contract version SHALL NOT change for this field.

#### Scenario: A malformed list is refused

- **WHEN** a provider answers the `capabilities` operation with a duplicate
  name, an out-of-shape name or more names than the bound
- **THEN** Forge refuses the delivery verb as an invalid provider response,
  names the reason, writes no delivery row and runs no requested operation

#### Scenario: A newer operation name is tolerated

- **WHEN** a provider declares a valid list that includes an operation this
  Forge does not know
- **THEN** Forge reads the operations it knows, ignores the rest, and does not
  refuse the provider

#### Scenario: A provider that declares nothing stays compatible

- **WHEN** a provider written before this change answers without a
  `capabilities` field
- **THEN** its request shape, its response requirements and the
  `forge-publish-provider/0.1.0` contract are unchanged for it
