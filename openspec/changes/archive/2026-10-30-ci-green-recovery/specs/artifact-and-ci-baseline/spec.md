## ADDED Requirements

### Requirement: CI installs every tool it invokes

Every Forge CI job SHALL install the external tools its steps invoke, at a
pinned version, or derive them from the workspace build. A missing tool SHALL
fail the job with an error naming it; no job SHALL skip a check because a tool
is absent.

#### Scenario: Validator is absent on a fresh runner

- **WHEN** the governance job starts without the OpenSpec CLI on `PATH`
- **THEN** it installs `@fission-ai/openspec@1.6.0` and runs strict validation,
  and a failed install fails the job rather than skipping the step

#### Scenario: Gate checker is absent on a fresh runner

- **WHEN** the gate job runs a plan whose `security` check invokes
  `cargo audit`
- **THEN** the job installs `cargo-audit` at a pinned version before running
  the Gate, and a failed install fails the job

### Requirement: The Gate job is bounded to its resolved plan

The CI gate job SHALL pass a time bound that covers the resolved plan's
per-check budgets. A wrapper timeout shorter than the plan SHALL NOT be used.

#### Scenario: Plan budgets exceed the default wrapper

- **WHEN** the plan grants `tests` and `declared-verification` 600 seconds each
- **THEN** the job runs `forge gate` with a bound of at least their sum plus
  the remaining checks, and the job mirrors the runtime's verdict

#### Scenario: Gate is blocked

- **WHEN** a bound check returns a blocking verdict
- **THEN** the job fails with that verdict and the recovery path, and no audit
  ignore or check exemption is added to make the job pass

### Requirement: The checker surface runs against a registered project

The surfaces job SHALL exercise `forge check` against a registered project. It
SHALL NOT assume that a fresh checkout has a registry row, because the checker
contract requires a registered target.

#### Scenario: Fresh checkout

- **WHEN** the surfaces job starts on a clean runner
- **THEN** it generates a scratch project under the runner's temporary
  directory, registers it and obtains an `alerts` document from `forge check`

#### Scenario: Registration or check fails

- **WHEN** generation, registration or the checker exits non-zero
- **THEN** the job fails and no partial document is treated as a pass

### Requirement: Rust sources are formatted and warning-free

The workspace SHALL pass `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --all-features -- -D warnings` at the
toolchain CI uses. A structural lint exception SHALL be a documented
`#[allow]` at the item, never a workflow-level suppression.

#### Scenario: Formatting drift

- **WHEN** any workspace source, including a vendored tooling crate, differs
  from rustfmt output
- **THEN** the formatting job fails and names the file

#### Scenario: Lint warning

- **WHEN** clippy emits a warning in any workspace target
- **THEN** the lint job fails, unless the item carries an explicit allow with
  its reason
