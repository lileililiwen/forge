## ADDED Requirements

### Requirement: Declare product source boundaries

Workspace Governance MUST require an applicable product-quality check to name
at least one product source root, a supported language profile, and explicit
exclusions for generated, vendor, fixture, or other non-product paths.

#### Scenario: Valid product policy

- **WHEN** a project declares an existing source root, supported language, and
  non-escaping exclusions
- **THEN** the checker scans the declared product scope

#### Scenario: Missing product policy

- **WHEN** a project enables the check without product roots or with an
  unsupported language
- **THEN** the result is `REVIEW_REQUIRED` with configuration remediation

### Requirement: Reject test code in product scope

The checker MUST report a blocking finding when test-only code is found inside
product scope, except for Rust test modules lexically enclosed by
`#[cfg(test)]`; repository test roots MUST remain outside product scope.

#### Scenario: Rust cfg test module

- **WHEN** a Rust product file contains a test module under `#[cfg(test)]`
- **THEN** the test module is allowed and produces no boundary finding

#### Scenario: Rust integration code in source

- **WHEN** a Rust product file contains an integration-test module or test-only
  imports outside `#[cfg(test)]`
- **THEN** the checker reports `product-test-code` as `FAIL`

#### Scenario: Non-Rust test module

- **WHEN** a Python, TypeScript, C#, or supported product file contains a
  recognized test-only module/file pattern
- **THEN** the checker reports `product-test-code` as `FAIL`

### Requirement: Enforce placeholder threshold

The checker MUST count configured placeholder markers only within product
scope and MUST return `FAIL` when the count exceeds the declared threshold.

#### Scenario: Threshold met

- **WHEN** the product marker count is at or below the configured threshold
- **THEN** the placeholder check passes and reports the count

#### Scenario: Threshold exceeded

- **WHEN** the product marker count is above the configured threshold
- **THEN** the checker reports `placeholder-threshold` as `FAIL` with per-file
  counts and remediation

#### Scenario: Excluded fixture marker

- **WHEN** a marker appears only in an explicitly excluded fixture or generated
  path
- **THEN** it does not count toward the product threshold

### Requirement: Emit bounded machine-readable findings

The checker MUST emit one versioned JSON document containing status, summary,
relative paths, line numbers, rule IDs, bounded diagnostics, and exit-code
compatible remediation without exposing configured secrets.

#### Scenario: JSON failure report

- **WHEN** a product boundary or threshold violation is found with `--json`
- **THEN** stdout contains one parseable report and the process exits 1

#### Scenario: Unreadable product file

- **WHEN** a declared product file cannot be read safely
- **THEN** the report is `REVIEW_REQUIRED` and the process exits 2
