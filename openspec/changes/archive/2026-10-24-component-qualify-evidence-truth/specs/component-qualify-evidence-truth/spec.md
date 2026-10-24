## ADDED Requirements

### Requirement: A component promotion records only supplied evidence

Forge SHALL record a component promotion only from evidence the operator
supplied. Forge SHALL NOT invent a test coverage, a verification timestamp
or a security review for a promotion. A promotion to `Certified` SHALL
require a test coverage, a last-verified timestamp and a security review; a
missing piece SHALL be a refusal that names every missing piece of
evidence, SHALL leave the prior quality level in place and SHALL NOT write
a receipt. A receipt SHALL carry only the evidence that was supplied.

#### Scenario: A certified promotion without evidence is refused

- **WHEN** a promotion to `certified` is requested without a test coverage,
  a last-verified timestamp and a security review
- **THEN** the promotion is refused, the refusal names the missing evidence,
  the prior quality level remains and no receipt is written

#### Scenario: A certified promotion with complete evidence is accepted

- **WHEN** a promotion to `certified` is requested with a test coverage at or
  above the certified minimum, a fresh last-verified timestamp and a
  recorded security review
- **THEN** the promotion is accepted and the receipt records exactly the
  supplied evidence

#### Scenario: A receipt carries no invented evidence

- **WHEN** a promotion receipt is written
- **THEN** it carries only the evidence fields the operator supplied, and it
  carries no default coverage, no default verification timestamp and no
  default security review

#### Scenario: One rule, every promotion writer

- **WHEN** a promotion is judged through the command line or through the core
  promotion path
- **THEN** both read the one certified evidence gate and both refuse a
  promotion whose evidence is absent
