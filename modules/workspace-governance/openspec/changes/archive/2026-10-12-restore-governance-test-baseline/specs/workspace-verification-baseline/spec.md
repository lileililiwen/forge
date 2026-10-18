## ADDED Requirements

### Requirement: The active Mnemora declaration is represented in the portfolio registry

The workspace registry SHALL contain exactly one entry for the active
`mnemora` declaration, with `id` and `path` set to `mnemora`, `profile` set to
`rust-product`, `role` set to `product`, and `stack` set to `rust`. The
registry entry SHALL agree with the declaration and SHALL NOT require a write
to the Mnemora repository.

#### Scenario: Mnemora is registered from its declaration

- **WHEN** the workspace audit or identity migration check reads the active
  Mnemora declaration
- **THEN** it finds one matching registry entry and reports no
  `DISCOVERED_UNREGISTERED` or taxonomy-axis mismatch for Mnemora

### Requirement: The promotion-test sandbox accounts for tracked governance data

The promotion-test sandbox SHALL copy every tracked top-level governance data
file that the workspace audit requires, or name it in the evidence-backed
absence table. Each tracked required file SHALL be copied into the temporary
sandbox before the audit runs. Missing or unaccounted files SHALL fail the
coverage test with their filenames.

#### Scenario: The three tracked records are copied

- **WHEN** the sandboxed promotion tests run with
  `registry-remote-drift.json`, `tag-taxonomy.json`, and
  `ui-adoption-migration.json` present at the governance root
- **THEN** all three files are copied into the sandbox and the coverage test
  passes without reading or writing the real workspace's promotion ledger
