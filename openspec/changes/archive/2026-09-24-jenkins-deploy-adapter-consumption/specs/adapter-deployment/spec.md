# adapter-deployment Specification

## ADDED Requirements

### Requirement: Versioned deploy executor contract

Forge SHALL define its deploy executor boundary as a versioned
`forge-deploy-executor/0.1.0` contract — fixed argv operations, a single
JSON envelope on stdout, and classification rules where a parseable
non-zero result is a failed stage while an unparseable or timed-out result
is unavailable with prior state preserved.

#### Scenario: Contract-conformant adapter

- **WHEN** an installed adapter emits a valid envelope for `apply`
- **THEN** Forge records the stage outcome exactly as the envelope names it
  and attributes the evidence to the adapter and its revision

#### Scenario: Contract-violating output

- **WHEN** an adapter emits non-JSON or an unknown contract version
- **THEN** the run is unavailable, the last good DeployState stands, and
  the refusal names the contract mismatch

### Requirement: Reference Jenkins adapter

Forge SHALL ship a tested reference adapter mapping its executor contract
onto the workspace's existing Jenkins deployment scripts (dry-run preview,
deploy, status) with an explicit health vocabulary where unrecognized
status reports `unknown` and never `healthy`.

#### Scenario: Dry-run rehearsal

- **WHEN** `forge provider run deploy --fixture` exercises the reference
  adapter with `--dry-run`
- **THEN** the row reports the sandbox fixture result without triggering
  any real deployment side effect

#### Scenario: Unrecognized status

- **WHEN** the underlying deployment reports a state outside the mapping
  table
- **THEN** observe returns `unknown` while the previous good observation
  remains the current_state history entry

#### Scenario: Production promotion is external

- **WHEN** the adapter is used against a real Jenkins host
- **THEN** success or failure is attributed to the provider run evidence,
  and no fixture-only claim is represented as production support
