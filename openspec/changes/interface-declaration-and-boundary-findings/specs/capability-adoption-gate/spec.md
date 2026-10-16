# capability-adoption-gate Specification (delta)

## ADDED Requirements

### Requirement: Boundary findings carry stable codes

Workspace Governance SHALL report a declared interface that violates the
declaration rules under a stable code, with the interface id and a remediation
naming the smallest lawful change, and SHALL validate the full declaration
before asserting anything about it so a malformed declaration never yields a
partial read. The codes SHALL be: `INTERFACE_DECLARATION_INVALID` (ERROR),
`INTERFACE_MECHANISM_UNKNOWN` (ERROR), `INTERFACE_CONTRACT_UNVERSIONED` (ERROR),
`INTERFACE_PROVIDER_UNDECLARED` (WARNING), `INTERFACE_CONSUMED_UNUSED`
(WARNING), `INTERFACE_SIBLING_PATH_UNPINNED` (WARNING) and
`INTERFACE_VENDORED_WITHOUT_DIGEST` (WARNING).

A one-sided edge — a consumption a provider does not declare — SHALL be reported
as `INTERFACE_PROVIDER_UNDECLARED` rather than treated as adopted, so an edge
counts as declared only when both sides declare it.

#### Scenario: One-sided edge

- **WHEN** a project declares a consumption whose provider declares no matching
  provision
- **THEN** governance reports WARNING `INTERFACE_PROVIDER_UNDECLARED` naming the
  provider and the interface id

#### Scenario: Malformed declaration yields no partial assertions

- **WHEN** a declaration fails schema validation
- **THEN** governance reports `INTERFACE_DECLARATION_INVALID` and asserts nothing
  else about that project's interfaces

### Requirement: Escalation of an interface finding is a recorded decision

An interface finding that begins as a WARNING SHALL NOT become blocking by
default, by an environment variable, or by a project-controlled switch. Its
escalation to FAIL SHALL be recorded as a decision in `decisions.json` with the
date it takes effect, following the existing recorded ramp rule, so tightening
the boundary is an owner decision that stays visible and revocable.

#### Scenario: Warning before the recorded date

- **WHEN** a repository carries a sibling edge without a digest and the recorded
  escalation date has not passed
- **THEN** the finding is a WARNING and does not block the gate

#### Scenario: Escalated after the recorded date

- **WHEN** the recorded escalation date has passed and the edge is still unpinned
- **THEN** the finding is a FAIL and the repository is blocked with the same
  remediation text

### Requirement: An unfrozen capability names its unverified provider

Governance SHALL report a capability registry entry whose posture is `exempt`
with reason `provider-unavailable` as WARNING `CAPABILITY_PROVIDER_UNVERIFIED`,
naming the provider that must declare verified evidence before the exemption can
lift. The reporter SHALL NOT change a posture, SHALL NOT charge a consumer for an
unfrozen capability, and SHALL NOT suppress the existing exemption semantics.

#### Scenario: Exempt entry reported

- **WHEN** a registry entry is exempt because its provider declares no verified
  evidence
- **THEN** governance reports `CAPABILITY_PROVIDER_UNVERIFIED` naming that
  provider, and the entry stays exempt

#### Scenario: Provider declares verified evidence

- **WHEN** the provider declares a verified capability record for that id
- **THEN** the reporter stops firing for that entry and the existing adoption
  rules apply unchanged
