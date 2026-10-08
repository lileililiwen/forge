# forge-workspace-sync Specification

## Purpose
Provide a single, repeatable terminal command that converges a workspace root into the Forge registry with an honest per-directory report, so a growing workspace of siblings can be brought under management in one run and re-converged safely on demand.
## Requirements
### Requirement: Repeatable workspace convergence

Forge SHALL provide `forge workspace sync [ROOT]` which brings every
immediate child directory of ROOT into the registry in one run: manifest
directories are registered, decidable directories are adopted via import
detection, already-registered directories report `already` without
rewriting, and every other directory reports an explicit skipped/failed
outcome with a reason. Re-running after adding siblings SHALL onboard only
the new ones; re-running a converged root SHALL report everything `already`.

#### Scenario: Sync onboards a mixed workspace

- **WHEN** the operator runs sync over a root with manifest, importable, ambiguous, undecidable and pre-registered directories
- **THEN** each directory gets exactly one honest outcome line plus a summary count, and the registry holds the onboardable set

#### Scenario: Rerun is idempotent

- **WHEN** sync runs twice over an unchanged root
- **THEN** the second run reports all previously onboarded directories as `already` and writes no new manifests

#### Scenario: Failures do not stop siblings

- **WHEN** one directory has an invalid manifest or an identity collision
- **THEN** it reports `failed` with the typed reason, every other directory is still processed, and the exit code is non-zero

### Requirement: Honest machine-readable report

Forge SHALL emit the same sync report as a human table and, with
`--format json`, as a `forge-workspace-sync/0.1.0` envelope carrying the
root, per-entry `{directory, outcome, id?, profile?, reason?}` and summary
counts, plus one counts-only `workspace.sync` journal row.

#### Scenario: JSON shape is stable

- **WHEN** sync runs with `--format json`
- **THEN** the envelope parses with the pinned contract, entry outcomes and summary counts

