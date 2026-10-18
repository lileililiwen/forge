# Proposal: Restore the governance verification baseline

## Why

The repository's declared unittest command currently fails for two concrete
reasons. The taxonomy migration check discovers the active `mnemora` project,
but `projects.json` has no corresponding registry entry. Separately, the
promotion-test sandbox coverage check finds three tracked governance data files
that its fixture does not copy. Both failures make the repository's required
verification command fail even though each has a bounded, evidence-backed fix.

## What Changes

- Register `mnemora` in this repository's `projects.json` using the id, path,
  role, stack, and profile already declared by its `.project.json`.
- Add the three missing tracked root data files to the promotion-test sandbox's
  copied data-file set.
- Add regression assertions for both conditions and record the measured
  verification results.

No sibling repository is written. Mnemora's declaration is read as evidence;
the only portfolio mutation is the registry owned by this repository.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `restore-governance-test-baseline` (this change) | Restore the repository's declared verification command to a passing state after two root-data drifts | `workspace-governance`, Python 3.11 stdlib | `projects.json` registration plus `tests/test_promote_project.py` sandbox inventory; no sibling writes | Existing registry-integrity and identity/taxonomy requirements | `python3 -m unittest discover -s tests -v` |

The two findings have different assertions, but share one repository owner,
verification lifecycle, and mandatory acceptance command. Neither repair can
be archived independently under this repository's completion gate while the
full suite remains red. They are therefore one small verification-baseline
repair, with separate tasks and regression checks for each root cause.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `workspace-governance` | `projects.json`, `scripts/workspace_check.py`, `tests/test_promote_project.py` | Owns the portfolio registry, audit, and sandbox fixture | The current root registry omits one declared active project; the fixture omits tracked root records | This repository; one local commit pair | **extend shared owner** |
| `driftwatchdog` | `.ai-gate/gate.yaml` integration | Runs checks selected by repository policy | It does not own project identity or Python test fixtures | Shared gate runtime; unchanged | **keep local** |
| `mnemora` | `.project.json` | Supplies the existing declaration used as registry evidence | Its source tree and declaration remain owned by that sibling | No sibling edit in this change | **keep local** |

## User Experience and Interface Impact

UI/UX: N/A. This change edits governance data and automated test fixtures; it
adds no human-facing product screen or interaction. Maintainers receive the
existing test and audit diagnostics.

## BFS Impact Map

- **Actors:** workspace registry maintainers and contributors running the
  declared verification command.
- **Data:** one registry row is added from Mnemora's declared identity and
  classification; three tracked JSON records become available in the temporary
  test sandbox.
- **Callers:** `axis_migration.py --check`, `workspace_check.py`, and the
  promotion tests. No public API, deployment, or package contract changes.
- **Failure behavior:** duplicate or conflicting identity remains a failing
  registry audit; missing sandbox files remain a failing test with the exact
  filename.
- **Privacy/security:** no secrets, credentials, or sibling source writes.
- **Rollback:** revert the implementation/archive commit; this removes the
  single registry row and fixture-list entries together.

## Capabilities

- Workspace project registry consistency.
- Repository-local governance test-fixture completeness.

## Non-goals

- Repairing Mnemora's other audit findings, including its archived Purpose
  placeholders.
- Changing any sibling declaration, gate manifest, or source file.
- Changing taxonomy migration semantics or the historical 81-project baseline
  recorded by the original migration fixture.
