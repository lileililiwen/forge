## MODIFIED Requirements

### Requirement: Emit bounded machine-readable findings

The checker MUST emit one versioned JSON document containing status, summary,
relative paths, line numbers, rule IDs, bounded diagnostics, and exit-code
compatible remediation without exposing configured secrets. The `readme-
status-claim` rule MUST participate in that same document, MUST report
findings by README line number and by contradicting evidence path, and MUST
pass every message through the existing redaction helper. Its statuses MUST
use the existing `PASS`, `FAIL` and `REVIEW_REQUIRED` vocabulary: a
contradicted claim is `FAIL`, an unresolvable claim is `REVIEW_REQUIRED`, and
a verified or absent claim produces no finding.

#### Scenario: JSON failure report

- **WHEN** a product boundary or threshold violation is found with `--json`
- **THEN** stdout contains one parseable report and the process exits 1

#### Scenario: JSON claim report

- **WHEN** a README claim is contradicted and the checker runs with `--json`
- **THEN** stdout contains one parseable report whose findings carry rule id
  `readme-status-claim`, the README line number, the contradicted surface,
  the evidence path, and no unredacted secret-shaped value

#### Scenario: Unreadable product file

- **WHEN** a declared product file cannot be read safely
- **THEN** the report is `REVIEW_REQUIRED` and the process exits 2

#### Scenario: An unresolvable claim is review-required, not a pass

- **WHEN** a claim names only surfaces with no decidable evidence rule
- **THEN** the report is `REVIEW_REQUIRED` and the process exits 2, and the
  report does not record the claim as verified
