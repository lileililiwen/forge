# platform-contract-consumption Specification

## MODIFIED Requirements

### Requirement: Digest-pinned vendored contract set

Forge SHALL vendor the published contract schemas, the contract registry and the
secret-field name list under `contracts/` behind a `contracts/manifest.json`
that records the source revision and the sha256 of every vendored file, and
SHALL verify those digests offline without reaching a sibling checkout or the
network. That internal record SHALL be sufficient for offline tamper detection
and SHALL NOT be treated as a statement of parity with the contract source:
Forge SHALL additionally compare the vendored bytes against the resolved
`platform-contracts` source, and for that comparison the digest authority SHALL
be the contract source's own record rather than Forge's self-referential
manifest.

#### Scenario: Vendored bytes match the manifest

- **WHEN** the digest test hashes every file listed in `contracts/manifest.json`
- **THEN** each recorded sha256 matches and the recorded source revision is
  reported

#### Scenario: Vendored file was edited locally

- **WHEN** a vendored schema's bytes differ from its recorded digest
- **THEN** the check fails naming the file, the expected digest and the actual
  digest, and no contract surface silently accepts the edited schema

#### Scenario: Contract set absent

- **WHEN** `contracts/manifest.json` is missing or unreadable and an operator
  requests a contract surface
- **THEN** the command refuses with a typed contract-invalid error naming the
  exact path, and no embedded or cached copy of the schemas is used instead

#### Scenario: Internal record agrees while the source has moved

- **GIVEN** a vendored file whose bytes match its entry in
  `contracts/manifest.json`
- **AND** the resolved `platform-contracts` source publishes different bytes for
  the same contract file
- **WHEN** the parity comparison runs
- **THEN** it fails naming the file and both digests, even though the offline
  manifest check passes
- **AND** the internal manifest's own digests are not consulted to decide the
  verdict

#### Scenario: Family digest is taken from the source's own record

- **GIVEN** a family the source's `manifest.json` declares with a `schema_digest`
- **WHEN** the parity comparison runs
- **THEN** the mirror's digest for that family is compared against the source
  manifest's `schema_digest` and the source's `registry_revision` is reported
- **AND** a source file the source manifest does not digest is still compared
  byte-for-byte

## ADDED Requirements

### Requirement: The parity gate compares the vendored families against the contract source

The contract parity check SHALL compare the actual bytes or the digests of every
contract file the vendored copy retains against the resolved `platform-contracts`
source, and SHALL compare every family the source's own record publishes, exiting
non-zero on any mismatch, on any family the source publishes that the mirror
does not retain, and on any retained file the source does not publish. The set
compared SHALL be derived from what the mirror retains and what the source's
record publishes rather than from a hard-coded list, and the check SHALL exclude
Forge's own `contracts/manifest.json` and the derived `contracts/vocabulary/`
tree from the parity decision.

#### Scenario: Mirror is byte-exact

- **GIVEN** a resolved `platform-contracts` source whose files are byte-identical
  to every contract file the vendored copy retains
- **WHEN** the parity check runs
- **THEN** it reports the compared file count and the source's
  `registry_revision` and exits zero

#### Scenario: Retained file drifted from the source

- **WHEN** a retained schema's bytes differ from the same file in the resolved
  source
- **THEN** the check exits non-zero naming the file and both digests
- **AND** it does not print a pass line

#### Scenario: Source publishes a family the mirror does not retain

- **GIVEN** a family the source's record publishes and the vendored copy does not
  retain a schema for
- **WHEN** the parity check runs
- **THEN** it fails naming the family and the schema the mirror lacks

#### Scenario: Mirror retains a file the source does not publish

- **GIVEN** a retained contract file with no counterpart in the resolved source
- **WHEN** the parity check runs
- **THEN** it fails naming the retained path as undeclared rather than skipping
  it

#### Scenario: Unpublished source work is not demanded of the mirror

- **GIVEN** a file present in the source working tree that the source's record
  does not publish as a family
- **WHEN** the parity check runs
- **THEN** it does not require the mirror to retain that file
- **AND** the mirror is required to carry it once the record publishes a family
  for it

#### Scenario: Every drifted file is reported in one run

- **GIVEN** two or more retained files that differ from the source
- **WHEN** the parity check runs
- **THEN** it names each one before exiting, rather than stopping at the first

### Requirement: A parity check that compared nothing is not a pass

The parity check SHALL count the comparisons it performed. A run that performs
zero comparisons SHALL exit non-zero and SHALL state explicitly that it verified
nothing. The check SHALL NOT print a pass line on any path that has not compared
the vendored bytes against the source, and the pass line SHALL be reachable only
when every comparison passed and at least one comparison was made.

#### Scenario: Nothing to compare

- **GIVEN** a resolved source and a mirror that yield no comparable file
- **WHEN** the parity check runs
- **THEN** it exits non-zero stating that it verified nothing
- **AND** it prints no pass line

#### Scenario: Pass is printed only after a real comparison

- **GIVEN** a run whose comparison count is greater than zero and whose failures
  are empty
- **WHEN** the parity check completes
- **THEN** it prints the pass line together with the comparison count

### Requirement: An unresolvable contract source is a hard failure

The parity check SHALL exit non-zero when no `platform-contracts` source can be
resolved, naming the resolution paths it tried. It SHALL NOT skip, SHALL NOT
print a pass, and SHALL NOT fall back to comparing the vendored copy against
itself or against Forge's own manifest.

#### Scenario: No sibling checkout and no variable

- **GIVEN** neither `PLATFORM_CONTRACTS_DIR` nor a `../platform-contracts`
  sibling checkout
- **WHEN** the parity check runs
- **THEN** it exits non-zero naming both resolution paths
- **AND** it does not print a pass line

#### Scenario: Source present but its own record is unusable

- **GIVEN** a resolved source whose `manifest.json` is missing, unparseable, or
  declares no family
- **WHEN** the parity check runs
- **THEN** it exits non-zero naming the source record it could not use

### Requirement: No input may weaken the comparison

`PLATFORM_CONTRACTS_DIR` SHALL locate the contract source and SHALL NOT have any
other effect on the comparison. The check SHALL provide no mode that skips,
softens, inverts or downgrades a comparison, and a failure SHALL NOT be
downgradable to a note. A resolved source that is the vendored tree itself SHALL
be refused, because comparing the mirror against itself cannot detect drift.

#### Scenario: Variable cannot turn the check off

- **WHEN** the check is invoked with any value of `PLATFORM_CONTRACTS_DIR`
- **THEN** the same files are compared with the same strictness
- **AND** no invocation reports a pass without a non-zero comparison count

#### Scenario: Variable points at the mirror

- **GIVEN** `PLATFORM_CONTRACTS_DIR` resolving to Forge's own `contracts/`
  directory
- **WHEN** the parity check runs
- **THEN** it exits non-zero naming the self-reference and does not report the
  trivially matching digests as a pass

#### Scenario: No advisory-only outcome

- **WHEN** a compared file mismatches
- **THEN** the check exits non-zero, and no flag, environment variable or
  configuration file can reduce that to a warning

### Requirement: The parity gate blocks release and CI

`scripts/release-check.sh` and the CI contract-parity job SHALL invoke the
parity check, and a mismatch SHALL block the run. Neither caller SHALL remove,
downgrade or bypass the check, and the check's exit status SHALL propagate to the
caller's exit status.

#### Scenario: Mismatch blocks the release check

- **GIVEN** a resolved source and a mirror that differs from it
- **WHEN** `scripts/release-check.sh` runs
- **THEN** the run stops at the parity step with a non-zero status and does not
  report a successful release check

#### Scenario: Mismatch blocks CI

- **GIVEN** a CI run whose contract-parity job reaches a mirror mismatch
- **WHEN** the job runs
- **THEN** the job fails

#### Scenario: The check is not removed from either caller

- **WHEN** the workflow and the release script are inspected
- **THEN** both still invoke the parity check, and no pre-existing job, gate or
  failure path has been removed or weakened
