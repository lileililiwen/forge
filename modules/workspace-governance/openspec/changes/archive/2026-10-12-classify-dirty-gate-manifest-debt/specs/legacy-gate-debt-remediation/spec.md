# legacy-gate-debt-remediation Specification (delta: classify-dirty-gate-manifest-debt)

## MODIFIED Requirements

### Requirement: Debt reasons are classified into safe remedy classes

The remediation driver SHALL classify each unresolved gate-debt entry into
classes A, B, C, D, E, F, G, or `?` based on its recorded reason. It SHALL apply
filesystem remedies only to A-D when the repository evidence satisfies that
remedy's preconditions. A class C repository without `[project]` distribution
metadata is escalated as a class C decision rather than being turned into a fabricated
package. E, F, G, and `?` SHALL be reported as decisions or escalations and SHALL
NOT be passed to a mechanical remedy. Class G SHALL identify a dirty gate manifest
as the reason a derived command remains unbound and direct the operator to review
and commit the manifest before rerunning the binder.

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

#### Scenario: Dirty manifest binding needs owner review

- **WHEN** the debt reason states that the binder left a derived command unbound because the gate manifest is dirty
- **THEN** the driver reports class G and instructs the operator to review and commit the manifest before rerunning the binder
- **AND** the driver does not invoke a remedy or modify the manifest

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

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|
| Safe classification | Report known debt reasons with safe owner actions | Exact dirty-manifest marker maps to reporting-only class G | `scripts/close_gate_debt.py:classify`, `Report._one` | known, unknown, decision, dirty manifest | DFS-1, DFS-2 | Classifier and no-write unit tests plus measured full-workspace dry run |
