# hypora-graduation-import Specification

## Purpose

Forge accepts a user-selected local `platform.idea-graduation` artifact
produced by Hypora and, only on explicit confirmation, seeds a project
from the validated brief. This capability owns the input contract
(`platform.idea-graduation/0.1.0`, major `0`), the closed key sets and
deny lists, the per-field bounds, the `validated: true` gate, the
read-only preview, and the minimal confirmed import that writes one
manifest and one receipt without scaffolding, deploying, publishing or
approving anything.

## Requirements
### Requirement: Forge validates graduation artifacts before use

Forge SHALL accept a user-selected local `platform.idea-graduation`
artifact only when its discriminator names the accepted contract family
with a supported major and a supported revision, when every object in
the artifact carries only keys from the closed key set for its level,
when every field is within its declared bound, and when the artifact
declares `validated: true`. Forge SHALL refuse an artifact before it
reads any artifact value into project state.

#### Scenario: Artifact conforms to the pinned contract

- **WHEN** an artifact declares
  `platform.idea-graduation/0.1.0`, carries only the closed key set at
  the artifact, brief, success-metric, experiment and evidence levels,
  stays within every bound, and declares `validated: true`
- **THEN** Forge maps it to a `GraduationImport` whose source names the
  family, the major, the revision, the Hypora project id, the Hypora
  revision and the graduation time

#### Scenario: Unsupported contract major

- **WHEN** the artifact's contract names a major Forge does not support
- **THEN** Forge refuses with `graduation-invalid`, naming the supported
  major, before creating any project or workspace state

#### Scenario: Unknown contract revision

- **WHEN** the artifact's contract names a revision outside the
  supported allowlist
- **THEN** Forge refuses with `graduation-invalid`, naming the supported
  revisions

#### Scenario: Artifact carries an out-of-set key

- **WHEN** any object in the artifact carries a key that is not in its
  closed key set
- **THEN** Forge refuses the whole import with `graduation-invalid`
  naming the key and the closed set, rather than dropping the key and
  reporting a clean import

#### Scenario: Artifact is not a validated graduation

- **WHEN** `experiment.validated` is absent or `false`
- **THEN** Forge refuses with `graduation-invalid` and the reason
  `graduation-not-validated`, and seeds no project

#### Scenario: A field exceeds its bound

- **WHEN** a title, brief field, requirement, success metric, evidence
  entry or provenance field exceeds its declared character bound
- **THEN** Forge refuses with `graduation-invalid` naming the field and
  the bound

#### Scenario: File is oversized or malformed

- **WHEN** the selected file exceeds the byte bound, is not valid UTF-8
  JSON, or is not a JSON object
- **THEN** Forge refuses with `graduation-invalid` before parsing it
  into a mapped record, and creates no project or workspace state

### Requirement: Import is previewed and explicitly confirmed

Forge SHALL show the mapped brief, the source provenance and the
aggregate evidence for an artifact before creating a project, and SHALL
create the project only when the operator repeats the command with an
explicit confirmation.

#### Scenario: Operator previews an artifact

- **WHEN** the operator runs `forge graduation preview <ARTIFACT>` on a
  valid artifact
- **THEN** Forge prints the mapped brief, the source provenance and the
  aggregate evidence and writes nothing

#### Scenario: Operator cancels before confirming

- **WHEN** a valid artifact has been previewed and the operator does not
  confirm
- **THEN** Forge creates no project, no directory, no manifest and no
  receipt

#### Scenario: Import without confirmation is a dry run

- **WHEN** the operator runs `forge graduation import <ARTIFACT>
  --path <DIR> --profile <PROFILE>` without `--confirm`
- **THEN** Forge validates the choices, prints the same preview with the
  resolved project identity and location, writes nothing and exits `0`

#### Scenario: Confirmed import creates one project and one receipt

- **WHEN** the operator runs the same import with `--confirm`
- **THEN** Forge writes a minimal `forge.yaml` and a
  `.forge/graduation/<id>/import.json` receipt, registers one project,
  and reports the project id, path and receipt path

### Requirement: Technical architecture remains user-selected

Forge SHALL require the operator to choose the stack/template
(`--profile`) and the destination directory (`--path`); the artifact
SHALL NOT name, imply or select either. Imported evidence SHALL NOT
scaffold, deploy, publish or approve a quality gate.

#### Scenario: Operator confirms without choosing a stack

- **WHEN** the artifact is valid but no `--profile` is supplied
- **THEN** Forge refuses the invocation and creates no scaffold and no
  project

#### Scenario: Artifact names no architecture

- **WHEN** the artifact is inspected
- **THEN** the accepted key set contains no profile, stack, template,
  language, repository or deployment field, so an architecture cannot be
  inferred from it

#### Scenario: Import does not scaffold or approve

- **WHEN** a confirmed import succeeds
- **THEN** the chosen profile is recorded on the project manifest, no
  profile template is applied, no deployment or publication occurs, and
  no quality gate is approved

### Requirement: Imported data is privacy-bounded and provenance is retained

Forge SHALL reject participant-level, raw-event, payment and
credential-bearing data by field name and by value shape, and SHALL
persist only the mapped project brief and the allowlisted source
provenance. Forge SHALL NOT retain the original artifact bytes or any
evidence excerpt, and SHALL NOT echo a refused value in an error, a
finding or a log.

#### Scenario: Artifact contains prohibited participant fields

- **WHEN** the artifact carries an identity, raw-event, payment or
  credential field, or a credential-, email- or query-URL-shaped value
  under an allowed name
- **THEN** the entire import is rejected with `graduation-invalid`
  before persistence and the offending value is never recorded

#### Scenario: Receipt carries only the allowlisted content

- **WHEN** a confirmed import writes its receipt
- **THEN** the receipt carries the contract, the project id, the source
  block, the mapped brief, the evidence **count**, the import time and
  the actor, and no other key

#### Scenario: Original artifact is not retained

- **WHEN** an import completes by any outcome
- **THEN** the original artifact bytes and every evidence excerpt are
  absent from the project directory, the receipt and every log, and the
  artifact file itself is left untouched

#### Scenario: Refusal names the rule, not the value

- **WHEN** Forge refuses an artifact
- **THEN** the error, in human and JSON form, names the field and the
  violated rule and contains no bytes of the refused value

### Requirement: Import failures are atomic

Forge SHALL leave no partial project, directory, manifest or receipt
when parsing, validation, choice resolution or persistence fails, and
SHALL report the failure as a typed error with empty stdout.

#### Scenario: Malformed JSON

- **WHEN** the selected file is malformed
- **THEN** Forge reports a redacted actionable error and creates no
  state

#### Scenario: Destination already holds a manifest

- **WHEN** the destination directory already contains `forge.yaml` or
  `platform.yaml`
- **THEN** Forge refuses with `graduation-conflict` and overwrites
  neither file; a duplicate import is only ever a new, separate project

#### Scenario: Registration fails after the files are written

- **WHEN** the manifest and receipt are written but registration fails
- **THEN** Forge removes the files it wrote and returns the typed error,
  leaving no half-adopted project

#### Scenario: Identity is reserved or already registered

- **WHEN** the requested id or the destination path is already
  registered
- **THEN** Forge refuses before writing anything and leaves the existing
  record unchanged

### Requirement: Graduation import stays local and side-effect-bounded

Forge SHALL perform graduation import without contacting Hypora or any
other network host, without invoking `gh`, and without mutating any
project other than the chosen destination and the registry.

#### Scenario: Import makes no network or gh call

- **WHEN** a preview or a confirmed import runs
- **THEN** no HTTP request is made, no `gh` process is spawned, and no
  Hypora credential is read or required; a preview spawns no process at
  all, and a confirmed import's only child process is the registry's
  existing best-effort `git` probe, unchanged from `forge import`

#### Scenario: Import touches exactly two surfaces

- **WHEN** a confirmed import succeeds
- **THEN** the only writes are the destination project directory and one
  registry row, and the registry gains no table or migration

#### Scenario: Unknown profile is refused read-only

- **WHEN** `--profile` names no existing profile descriptor
- **THEN** Forge refuses with `unknown-profile` and writes nothing

#### Scenario: Artifact path is unavailable

- **WHEN** the artifact path cannot be read or the destination parent
  does not exist
- **THEN** Forge refuses with `path-unavailable` and writes nothing

