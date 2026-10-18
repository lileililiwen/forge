# legacy-gate-debt-remediation Specification (delta: drive-legacy-gate-debt-to-zero)

## ADDED Requirements

### Requirement: Debt reasons are classified into safe remedy classes

The remediation driver SHALL classify each unresolved gate-debt entry into
classes A, B, C, D, E, F, or `?` based on its recorded reason. It SHALL apply
filesystem remedies only to A-D when the repository evidence satisfies that
remedy's preconditions. A class C repository without `[project]` distribution
metadata is
escalated as a class C decision rather than being turned into a fabricated
package. E, F, and `?` SHALL be reported as decisions or escalations and SHALL
NOT be passed to a mechanical remedy.

#### Scenario: Known mechanical reason is classified

- **WHEN** the debt reason reports that a checker launcher is absent
- **THEN** the entry is class A and the dry-run names the checker remedy
- **AND** class B, C, and D reasons map to their corresponding classes

#### Scenario: An unknown reason is encountered

- **WHEN** a debt reason matches none of the known classifiers
- **THEN** it is reported as class `?` with the original reason
- **AND** no repository file is written for that entry

#### Scenario: A product decision is unresolved

- **WHEN** a debt entry has multiple equally plausible product entry points or
  no derivable build command
- **THEN** it is reported as class E or F with the decision required
- **AND** no mechanical remedy runs

#### Scenario: Python source has no package metadata

- **WHEN** class C identifies Python source but the repository has no
  `[project]` table in `pyproject.toml`
- **THEN** the driver asks whether the repository publishes a Python
  distribution
- **AND** it does not create package metadata or claim the build debt is closed

#### Scenario: A dirty manifest prevents a derivable binding

- **WHEN** the binder can derive a command but refuses to write because the
  gate manifest is dirty
- **THEN** the debt record retains that concern as currently unbound
- **AND** the record states that the command remains unbound because the
  manifest is dirty

### Requirement: Dry-run is safe and apply is repeatable

The driver SHALL make no filesystem changes by default. With `--apply`, it
SHALL apply only the selected A-D remedy and SHALL make repeated application
idempotent for generated launchers, solution membership, build metadata, and
security-tool declarations.

#### Scenario: Default invocation previews changes

- **WHEN** the driver runs without `--apply`
- **THEN** it reports proposed remedies and leaves all target files byte-for-byte
  unchanged

#### Scenario: A solution already contains some projects

- **WHEN** class B runs against a repository with an existing `.sln` or `.slnx`
- **THEN** it adds only missing project paths
- **AND** it verifies every discovered project is represented in the solution

#### Scenario: A class A-D remedy is applied more than once

- **WHEN** the same debt record is applied again after its artifacts exist
- **THEN** the driver reports the existing state without duplicating entries or
  declarations

### Requirement: Mechanical remedies are evidenced and fail closed

The driver SHALL verify generated artifacts and SHALL report command failures,
unreadable inputs, and incomplete outcomes without claiming the concern passes.
The gate-command binder SHALL derive a Python build command only when the
repository declares a `[build-system]` and has the build tool available through
its declared development dependencies or runtime environment.

#### Scenario: The .NET CLI reports an add error with exit zero

- **WHEN** `dotnet sln add` exits zero but its output says the project is
  missing or cannot be added
- **THEN** the remedy reports failure and does not count the project as added

#### Scenario: An empty solution is generated

- **WHEN** a solution file exists but contains no project entries
- **THEN** class B reports incomplete remediation
- **AND** it does not claim the build check is closed

#### Scenario: Python package metadata can build

- **WHEN** a Python package declares `[build-system]` and the `build` frontend
  in its project or development dependencies
- **THEN** the binder derives `python -m build` for its build concern
- **AND** the driver report identifies that command as the remedy outcome

#### Scenario: Python package metadata cannot build

- **WHEN** `[build-system]` is absent or the `build` frontend is undeclared
- **THEN** the binder leaves the build concern unresolved with a reason
- **AND** it does not emit a command that is known to be unavailable

#### Scenario: A dependency audit cannot reach its index

- **WHEN** the class D remedy declares `pip-audit` but invocation fails because
  the tool or network is unavailable
- **THEN** the report preserves the failure detail and does not label the audit
  as passed

### Requirement: Shared checkers report actionable findings

Each bound checker SHALL perform the concern-specific scan, return 0 when clean,
1 when findings exist, and 2 when it cannot complete. Every finding SHALL
identify a repository-relative file and line. A checker SHALL NOT suppress a
finding because its output is inconvenient to the repository owner.

#### Scenario: A source finding is detected

- **WHEN** a repository contains a concrete concern violation
- **THEN** the checker exits 1 and names the relative file and line

#### Scenario: Relevant input cannot be read

- **WHEN** the checker cannot read a required source file or parse a required
  manifest
- **THEN** it exits nonzero and identifies the failed input

#### Scenario: A concern has no applicable files

- **WHEN** a checker cannot find any relevant product surface for its concern
- **THEN** it reports `NOT_APPLICABLE` with the searched surface in its notes
- **AND** the checker exits 0 without reporting a clean scan of files it did not inspect

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|
| Safe classification | Classify six reason families and unknowns | Reason-string classifier with escalation | `scripts/close_gate_debt.py:classify`, `Report._one` | known, unknown, decision | DFS-1 | classifier unit tests and 24-entry distribution |
| Dry and idempotent remedies | Preview by default; apply A-D safely | explicit `--apply`, per-remedy idempotency | `scripts/close_gate_debt.py:main`, `remedy_a`-`remedy_d` | dry, partial solution, repeated apply | DFS-2 | temp-repo before/after and repeated-run tests |
| Fail-closed evidence and Python build | Verify output, allow only derivable commands | parse `.slnx`/`.sln`, Python build tool detection | `scripts/close_gate_debt.py:remedy_b`; `scripts/bind_gate_commands.py:python_candidates`, build resolution | exit-zero error, empty solution, build eligible/ineligible, audit unavailable | DFS-3 | binder tests, solution fixtures, command capture |
| Actionable checker findings | Findings have path/line and fail closed | one central concern engine, thin launchers | `scripts/check_concern.py`, product `scripts/check-*.py` | violation, unreadable input | DFS-4 | checker unit/real repo invocations |
| Dirty manifest debt remains live | Do not omit checks whose writes are blocked | DIRTY outcomes include derivable-but-unwritten checks | `scripts/bind_gate_commands.py:debt_concerns`, `build_debt` | dirty manifest, gate oracle parity | DFS-1, BFS-3 | fixture for dirty manifest and full gate-unbound set comparison |
