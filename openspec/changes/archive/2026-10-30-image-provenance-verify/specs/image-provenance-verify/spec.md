## ADDED Requirements

### Requirement: An image build record is verified against the current revision

Forge SHALL provide `forge build --verify-image`, which resolves the project's
committed Git revision and reads
`.forge/build/<project-id>/<sha12>.json`. The record SHALL be refused with
`build-invalid` when it is absent, unreadable, carries a contract whose family
is not `forge-build-image` or whose major is not `0`, names a different project
or revision, or carries a digest that is not `sha256:` followed by 64
lower-case hex characters. Verification SHALL write no file and SHALL start no
builder.

#### Scenario: A fresh record verifies

- **WHEN** the record for the revision at `HEAD` names this project and carries
  a valid digest
- **THEN** Forge exits zero and reports the record and `verified: true`

#### Scenario: The revision has moved

- **WHEN** the project's `HEAD` is a revision with no record
- **THEN** Forge refuses with `build-invalid`, names the record path and the
  revision, and says to run `forge build --image` at this revision

#### Scenario: The record belongs to another project

- **WHEN** the record's `project_id` differs from the project's id
- **THEN** Forge refuses with `build-invalid` and writes nothing

#### Scenario: The record carries an unsupported contract

- **WHEN** the record declares `forge-build-image/1.0.0`
- **THEN** Forge refuses with `build-invalid` and names both contracts

#### Scenario: The digest is not a sha256 digest

- **WHEN** the record's digest is absent or malformed
- **THEN** Forge refuses with `build-invalid`

### Requirement: A changed SBOM invalidates the record

When the record carries an SBOM, Forge SHALL re-apply the build's SBOM path
rule, read the file, and compare its SHA-256 and byte length with the recorded
values. A file that is missing, outside the project or over the size bound
SHALL be refused with `build-invalid`; a file whose bytes differ SHALL be
refused with `build-failed`, naming the recorded and observed hashes.

#### Scenario: The SBOM is unchanged

- **WHEN** the SBOM is present, inside the project and hashes to the recorded
  value
- **THEN** verification succeeds

#### Scenario: The SBOM was edited after the build

- **WHEN** the SBOM's bytes no longer hash to the recorded SHA-256
- **THEN** Forge refuses with `build-failed` and names both hashes

#### Scenario: The SBOM was deleted

- **WHEN** the record names an SBOM that no longer exists
- **THEN** Forge refuses with `build-invalid` and writes nothing

### Requirement: Verification is a bindable Gate check and nothing more

`--verify-image` SHALL be exclusive with `--image`, `--tag`, `--builder-bin`,
`--os`, `--arch` and `--rid`, and each combination SHALL be refused with
`build-invalid`. On success the JSON document SHALL carry the record under
`image_build` and `verification: {verified: true, revision, digest}`. On
failure the process SHALL exit non-zero with its typed code, so a project can
bind it as a declared `source: bound` check without any Gate engine change.
Forge SHALL NOT add the check to the baseline or to a pack.

#### Scenario: A project opts in

- **WHEN** a project declares an `image-provenance` check with `source: bound`
  and a script that runs `forge build . --verify-image`
- **THEN** the Gate's verdict for that check is the verb's exit status and
  bounded output, with no other Forge change

#### Scenario: Verification is combined with a build

- **WHEN** `--verify-image` is supplied with `--image` or a build flag
- **THEN** Forge refuses with `build-invalid` and starts nothing

#### Scenario: A project does not opt in

- **WHEN** a project declares no image-provenance check
- **THEN** its Gate is unchanged and no image record is required
