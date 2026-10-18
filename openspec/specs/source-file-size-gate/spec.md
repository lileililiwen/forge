# source-file-size-gate Specification

## Purpose
TBD - created by archiving change forge-workspace-governance-gate-conformance. Update Purpose after archive.
## Requirements
### Requirement: Workspace Governance scripts are explicitly scanned

The Workspace Governance module SHALL configure its native `gate.toml` source-size policy to include `scripts/**/*.py`. Forge's conventional source roots and other projects' default candidate sets SHALL remain unchanged. The module SHALL retain the existing 1,000-line default, exclusion behavior, and failure semantics.

#### Scenario: Module Python scripts are evaluated by the built-in baseline

- **WHEN** `forge gate modules/workspace-governance` resolves the module's native `gate.toml`
- **THEN** the source-size check SHALL count that file as an evaluated candidate and report its measured candidate total

#### Scenario: An oversized module script fails the baseline

- **WHEN** a non-excluded Python file under `modules/workspace-governance/scripts/` has 1,001 raw newline characters
- **THEN** the source-size check SHALL fail with a bounded finding naming `scripts/sample.py`, its line count, and the 1,000-line maximum

#### Scenario: Module-local include does not change other projects

- **WHEN** another Forge project has no explicit source-size include for scripts
- **THEN** its candidate set SHALL remain governed by the existing conventional roots and exclusions

### Requirement: Consolidated Python governance sources meet the Forge baseline

The maintained Python source files under `modules/workspace-governance/scripts/` SHALL remain at or below 1,000 raw newline characters each and SHALL be evaluated by Forge Gate without source-path exclusions or a raised threshold.

#### Scenario: Module Gate checks real Python candidates

- **WHEN** `forge gate modules/workspace-governance` runs after this change
- **THEN** the `source-file-size` evidence SHALL report a nonzero evaluated candidate count for the module's `scripts/` source tree

#### Scenario: Refactored scripts preserve their entry points

- **WHEN** an existing module test or documented invocation imports or runs one of the four split scripts
- **THEN** the original path SHALL remain importable/executable and preserve its documented arguments, output shape and exit behavior

#### Scenario: Installed Forge carries split checker dependencies

- **WHEN** Forge is packaged and installed, then `forge concern check product-code-boundary` runs from the installed binary
- **THEN** the archive and installer SHALL place every helper imported by `product_code_quality.py` beside that script, and the check SHALL execute without relying on the source checkout

#### Scenario: No maintained script exceeds the limit

- **WHEN** the module's full Gate runs
- **THEN** it SHALL pass the source-size check with no source-size findings and no excluded oversized maintained source file
