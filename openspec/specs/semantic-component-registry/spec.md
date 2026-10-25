# semantic-component-registry Specification

## Purpose
Semantic reusable-component registry below feature granularity: meaningful units with explicit inputs/outputs, tests, versions, compatibility and deterministic installation, under quality levels (experimental/verified/certified/deprecated) with certified preferred and deprecated refused. A promotion records only the evidence the operator supplied: an absent measurement is never a default value, and a certification is refused unless the coverage, the verification timestamp and the security review it requires were supplied. A resolution reports the plan it produced and is never recorded as a completed installation.
## Requirements
### Requirement: Semantic reusable unit contracts

Forge SHALL register meaningful components with explicit inputs/outputs, tests, versions, compatibility and deterministic installation, linked to features, patterns and capabilities. A resolution SHALL report the plan it produced and SHALL NOT record a completed resolution, because a resolution installs nothing.

#### Scenario: Semantic reusable unit contracts success

- **WHEN** a PaginatedQuery or IdempotencyGuard component satisfies the contract
- **THEN** its version can be discovered and resolved for supported profiles

#### Scenario: Semantic reusable unit contracts failure

- **WHEN** an asset is merely a language primitive or lacks required contracts
- **THEN** registration rejects it with specific missing criteria

#### Scenario: Semantic reusable unit contracts boundary

- **WHEN** two stacks implement the same semantic capability
- **THEN** each preserves its own implementation while exposing the shared contract

#### Scenario: A resolved component plan is not an installation

- **WHEN** a resolution produces a plan for a profile
- **THEN** the journal records a planned resolution, the detail names
  the run, and the plan is not recorded as a completed installation

### Requirement: Quality classification and selection

Forge SHALL track Experimental, Verified, Certified and Deprecated states with supporting evidence and prefer compatible Certified components without violating constraints. Forge SHALL record a promotion only from evidence the operator supplied: an absent measurement is never a default value, and a promotion to Certified SHALL be refused unless the operator supplied a test coverage, a last-verified timestamp and a security review.

#### Scenario: Quality classification and selection success

- **WHEN** compatible certified and experimental candidates satisfy the same request
- **THEN** the resolver prefers the certified candidate and reports its evidence

#### Scenario: Quality classification and selection failure

- **WHEN** certification lacks required verification or security evidence
- **THEN** promotion fails and the prior quality level remains

#### Scenario: Quality classification and selection boundary

- **WHEN** only a deprecated or incompatible certified candidate exists
- **THEN** the resolver reports the policy conflict rather than silently selecting it

#### Scenario: A certification is never claimed without its evidence

- **WHEN** a promotion to certified is requested without the evidence a
  certification requires
- **THEN** the promotion is refused, the refusal names the missing evidence,
  the prior quality level remains and no receipt is written

