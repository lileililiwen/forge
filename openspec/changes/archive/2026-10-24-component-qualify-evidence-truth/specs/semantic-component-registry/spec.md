## MODIFIED Requirements

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
