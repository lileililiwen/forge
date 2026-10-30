# forge-build-image Specification

## Purpose
`forge build` produced no artifact Forge could name: the deploy lane's artifact
is a container file the project owns, and nothing recorded which image came out
of which revision. This capability adds `forge build --image`, which delegates
the build to one operator-installed adapter under
`forge-image-builder/0.1.0`, validates every claim the adapter makes (the
requested tag, a `sha256` digest, a bounded identity, an SBOM path inside the
project), hashes the SBOM itself, and writes one revision-bound provenance
record under `.forge/build/<project-id>/<sha12>.json`. Forge never installs,
downloads or vendors a builder, and the record is its own evidence rather than
Gate evidence.
## Requirements
### Requirement: An image build is delegated and bound to its revision

Forge SHALL provide `forge build --image`, which resolves the project's Git
revision, derives a default image tag of `forge-<project-id>:<sha12>` unless
the operator supplies `--tag`, and invokes one operator-installed image-builder
adapter through the `forge-image-builder/0.1.0` stdin/stdout contract. Forge
SHALL NOT install, download or vendor a builder. The adapter SHALL be resolved
from `--builder-bin`, then `$FORGE_IMAGE_BUILDER_BIN`, then
`forge-image-builder` on `PATH`.

#### Scenario: An image is built

- **WHEN** the adapter answers `status: built` with the requested image and a
  `sha256` digest
- **THEN** Forge writes one provenance record under
  `.forge/build/<project-id>/<sha12>.json` and reports the tag, digest and
  builder identity

#### Scenario: No builder is configured

- **WHEN** the resolved adapter binary cannot be started
- **THEN** Forge refuses with `build-invalid`, names the binary and the
  variable, and says Forge never installs a builder

#### Scenario: The project has no revision

- **WHEN** the project directory has no Git `HEAD`
- **THEN** Forge refuses with `build-invalid` and writes no record

#### Scenario: The image path is combined with target flags

- **WHEN** `--image` is supplied with `--os`, `--arch` or `--rid`
- **THEN** Forge refuses with `build-invalid`, because the image platform
  belongs to the builder

#### Scenario: An image flag is used without the image path

- **WHEN** `--tag` or `--builder-bin` is supplied without `--image`
- **THEN** Forge refuses with `build-invalid`

### Requirement: Forge refuses a build claim it cannot check

Forge SHALL refuse an adapter response whose contract family is not
`forge-image-builder` or whose major is not `0`, whose status is neither
`built` nor `failed`, whose image does not equal the requested tag, whose
digest is not `sha256:` followed by 64 lower-case hex characters, whose builder
identity is empty or longer than 128 characters, or whose evidence exceeds
twenty lines or 800 characters per line. A response whose SBOM path is
absolute, leaves the project through `..`, names no file, or names a file
larger than 8 MiB SHALL also be refused. Forge SHALL compute the SBOM's
SHA-256 itself and SHALL NOT record a hash supplied by the adapter.

#### Scenario: The adapter built something else

- **WHEN** the response's image differs from the requested tag
- **THEN** Forge refuses with `build-invalid` and writes no record

#### Scenario: The digest is not a sha256 digest

- **WHEN** the response's digest is absent or not
  `sha256:<64 lower-case hex>`
- **THEN** Forge refuses with `build-invalid` and writes no record

#### Scenario: The SBOM path escapes the project

- **WHEN** the response's `sbom_path` is absolute, contains a `..` component,
  or resolves outside the project directory
- **THEN** Forge refuses with `build-invalid` and writes no record

#### Scenario: The SBOM exists

- **WHEN** the response names a project-relative SBOM file that exists and is
  within the size bound
- **THEN** Forge records its project-relative path, its byte length and the
  SHA-256 Forge computed

#### Scenario: A newer contract minor is read

- **WHEN** the adapter declares `forge-image-builder/0.2.0`
- **THEN** Forge accepts the response under the shared version rule

### Requirement: A failed image build is a typed failure

A response with `status: failed`, a non-zero adapter exit, or an adapter that
exceeds the bounded timeout SHALL be reported as `build-failed` with the
adapter's evidence, and SHALL write no provenance record. A partially written
record SHALL never be visible.

#### Scenario: The adapter reports failure

- **WHEN** the adapter answers `status: failed` with evidence
- **THEN** Forge reports `build-failed`, includes the evidence, and writes no
  record

#### Scenario: The adapter exits non-zero

- **WHEN** the adapter exits non-zero without a readable response
- **THEN** Forge reports `build-failed` with the captured output and writes no
  record

#### Scenario: The adapter hangs

- **WHEN** the adapter exceeds the shared native-command bound
- **THEN** Forge kills the process group and reports `build-failed`

#### Scenario: A record is replaced

- **WHEN** a build for the same project and revision succeeds twice
- **THEN** the second record replaces the first atomically and no temporary
  file remains

