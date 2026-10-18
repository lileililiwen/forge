# Design: Restore the governance verification baseline

## 1. Implementation boundary

Repository: `/home/paul/code/workspace-governance`, Python 3.11, standard
library only.

Change only:

- `projects.json` — append one registry record for the existing active
  declaration `mnemora/.project.json`.
- `tests/test_identity_and_taxonomy.py` — pin the post-migration registry
  addition without rewriting the historical migration baseline.
- `tests/test_promote_project.py` — include all three missing tracked root
  records in the sandbox's declared copied set.
- This change's OpenSpec artifacts and `HANDOFF.md` evidence.

Do not write any sibling repository. Do not regenerate or rewrite the original
identity migration snapshot: it records the earlier 81-declaration migration
baseline and remains historical evidence.

## 2. Language and runtime

Python 3.11 standard library. Verification commands:

- `python3 -m unittest tests.test_identity_and_taxonomy tests.test_promote_project -v`
- `python3 -m unittest discover -s tests -v`
- `python3 scripts/workspace_check.py --self-check`
- `python3 scripts/auto_gate.py --self-check --no-deployment`
- `python3 scripts/axis_migration.py --root /home/paul/code --governance-root . --check`
- `node scripts/check-openspec-change-names.mjs`
- `openspec validate --all --strict --no-interactive`
- `git diff --check`

## 3. Ownership and shared code

`workspace-governance` owns `projects.json` and the sandbox fixture. The
portfolio registry is the existing shared authority for project identity;
Mnemora's declaration supplies the row values. `driftwatchdog` owns gate
execution and has no registry or test-fixture implementation to reuse. No
shared package or sibling source change is justified.

## 4. Registry behavior

The new record is exactly:

| Field | Value | Evidence |
|---|---|---|
| `id` | `mnemora` | `.project.json` |
| `path` | `mnemora` | workspace directory and declaration location |
| `profile` | `rust-product` | `.project.json` |
| `role` | `product` | `.project.json` |
| `stack` | `rust` | `.project.json` |
| `lifecycle` | `active` | `.project.json` |
| `adoption` | `adopted` | existing registry convention for registered projects |

The record clears `DISCOVERED_UNREGISTERED` and gives the taxonomy migration
check the matching canonical axes. It does not change the declaration,
`kind`, project requirements, or Mnemora's other audit findings. The original
identity migration fixture remains an 81-project historical snapshot; the
regression test records Mnemora as a later addition.

## 5. Sandbox behavior

`tests/test_promote_project.py` already copies a declared tuple of
fail-closed governance data files into an isolated temporary root. Extend that
tuple with:

- `registry-remote-drift.json`
- `tag-taxonomy.json`
- `ui-adoption-migration.json`

The existing coverage test discovers top-level JSON files and requires each to
appear exactly in the copied set or the evidence-backed absent set. These three
files are tracked governance inputs, so copy them; do not classify them as
absent. The temporary sandbox remains isolated from the real registry and
promotion ledger.

## 6. Compatibility and failure policy

- Duplicate `mnemora` IDs or paths continue to fail the existing registry
  checks; do not normalize the id or infer a path.
- An axis disagreement is not hidden by this row; the registry copies the
  declaration's explicit canonical values.
- A missing data file in the sandbox remains a hard test failure naming that
  file.
- No runtime, deployment, or release claim follows from these tests.

## 7. Verification oracle

The exact regression oracle is the repository's declared unittest command.
Before implementation, it reproduced two failures: the live taxonomy check
reports Mnemora's missing registry row, and the sandbox coverage test names the
three omitted tracked files. After implementation, both focused test modules
and the complete suite must pass. The axis migration check must report no
outstanding rows. The workspace self-check and unattended auto-gate must pass.

## 8. Decision ledger

- **Resolved:** register Mnemora from its declaration; do not edit its sibling
  repository.
- **Resolved:** copy the three tracked governance records into the test
  sandbox; do not add them to the absence table.
- **Deferred:** Mnemora's archived Purpose placeholders and all other Mnemora
  findings remain for their owner and are outside this package.
- **Resolved:** keep the original migration totals and baseline fixture
  historical; assert the later registry addition separately.
