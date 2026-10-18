# Design: Repair mechanically classifiable legacy gate debt

## Implementation boundary

Owner: `/home/paul/code/workspace-governance`, Python 3. The public CLI is `scripts/close_gate_debt.py`; shared checks live in `scripts/check_concern.py`; derivation remains in `scripts/bind_gate_commands.py`. Tests live in `tests/test_close_gate_debt.py` and existing binder tests. Explicit `--apply` may write only the repository paths selected by the debt record under the supplied workspace root. The script does not run from sibling repositories and does not alter product documentation.

## Language and verification environment

Python 3.11+ (stdlib-only driver/checker core); `dotnet` is invoked for class B; `pip-audit` and `python -m build` are gate tools for their respective concerns. Local verification: `python3 -m unittest tests.test_close_gate_debt -v`, `python3 -m unittest tests.test_bind_gate_commands -v`, and `python3 -m unittest discover -s tests -v`. Strict spec validation: `openspec validate --changes --strict`.

## Ownership and shared code

`workspace-governance` owns classifier, driver, and checker logic because they implement portfolio policy and are consumed by its registered projects. Product repositories get small launchers that resolve the canonical governance path relative to their own location. `driftwatchdog` continues to own gate execution and supplies the debt contract; it does not gain repair logic. `forge` remains the orchestration owner and is not called by this local repair CLI. This is an adapter to the existing debt record, not a new shared wire contract.

## Behavior and class model

Input is `workspace-governance/gate-debt.json`, a list of repositories and unresolved `{concern, reason}` pairs. The driver resolves the workspace root, reads the record once, and classifies each reason using the six class predicates. Dry-run is default. `--apply` runs only classes A-D. `--repo` limits scope, and `--json` serializes the same outcomes. A reason outside the classifier is class `?` and escalates; classes E/F are known decisions and also escalate. None are passed through to a mechanical remedy.

The binder's generated debt record also describes actual gate state for dirty manifests. If the binder can derive a command but its write guard refuses a dirty manifest, that concern remains in the record with the derived command and a dirty-manifest reason. It is not treated as repaired until a later successful bind writes the command.

| Class | Match evidence | Remedy | Completion oracle |
|---|---|---|---|
| A | Binder says a `scripts/check-<concern>.*` rule exists but file is absent | Write a thin launcher only for one of the five implemented concerns | Launcher invokes central checker; binder resolves command; checker executes and reports findings |
| B | `.csproj` paths exist and no solution is declared | Find/create `.sln` or `.slnx`, add missing projects, verify every project path is represented | Parse solution entries and compare normalized project paths with discovered `.csproj` paths |
| C | Binder reports pure Python with no build step | For an existing Python distribution, require `[project]` metadata, ensure `pyproject.toml` declares a build backend, and declare the `build` frontend; binder derives `python -m build`. Without distribution metadata, ask the owner | Binder command and an isolated build invocation agree with the package metadata |
| D | No Python security tool declared | Add `pip-audit` to `requirements-dev.txt` if absent and invoke it | Existing/non-existing dependency declaration is reported accurately; nonzero/network failure remains visible |
| E | More than one equally plausible entry point | No write; ask which directory is the product | Entry remains in debt |
| F | No build script/command can be derived | No write; ask what the repository builds | Entry remains in debt |
| ? | No predicate matches | No write; report exact reason and require a new class | Entry remains visible and is never passed to A-D |

Reapplying a mechanical class is idempotent: no duplicate solution projects, checker launchers, build-system tables, or pip-audit declarations. Remedy errors are attached to that entry and do not crash the entire run. A successful file write is not proof the concern passes; the binder and checker must be rerun.

## Contracts and compatibility

The driver keeps the current JSON report envelope and exit behavior: `--json` returns structured results; default text prints one row per debt item. The input debt schema is unchanged. No command binding is overwritten; the existing binder remains the sole writer of generated gate commands. The only added Python build derivation is `python -m build` for a declared build system with an available/declared `build` tool; absence of a usable tool remains unresolved instead of generating a guaranteed-failing command.

Checker launcher invocation is `[sys.executable, <central-checker>, <concern>, <repo-root>]`, with no shell interpolation. Checker exit values remain 0 clean, 1 findings, 2 unreadable/invalid invocation. Findings include repository-relative path and line. The central engine fails closed for unreadable relevant files and ignores dependency/build output directories.

## Failure and boundary behavior

- Missing debt record: CLI exits 2 and names its path.
- Invalid JSON: CLI exits 2 with parse context; no remedy runs.
- Missing/invalid repository identity or absent repository: the entry is reported as skipped/refused, never silently counted as repaired.
- Class C without `[project]` distribution metadata: ask whether the Python source is a distributable package; do not invent package metadata for an application or operations repository.
- Unknown class: no writes; explicit `?` escalation.
- `dotnet` failure: inspect both exit code and diagnostic output; never assume exit 0 means success. Preserve any created solution and report incompleteness for safe rerun.
- Existing solution: add only missing projects; `.slnx` `<Project Path="…" />` entries and legacy `.sln` project records are both parsed.
- Audit/build tool unavailable or network failure: report the declared binding separately from its runtime result; do not call the audit passed.
- Checker unreadable input: exit 2 or emit a finding with source location; never return clean.
- Secrets: no credentials are written, logged, or needed.

## Verification oracle

Unit tests use temporary repositories and synthetic debt records. They assert all six classes plus `?`, dry-run no writes, `--repo` filtering, JSON/text stability, each remedy's observable artifacts, repeat-run idempotency, and failure reporting. Class B has both `.slnx` and `.sln` fixtures and verifies completeness, including the missing-project negative control. Class C proves the binder refuses absent metadata/tooling and derives the declared build command when eligible. Class D verifies an existing declaration is still run and no duplicate is added. Real-workspace verification begins with the 24-entry distribution above, runs dry mode, applies A-D, executes every generated checker, regenerates the debt record with `--debt-out` at governance root, and measures remaining decisions. Findings are reported as findings, never fixed by suppressing or editing product prose.

## Decision ledger

- Keep the reason-string classifier because it follows the existing binder's refusal interface and unknown reasons escalate. Tests pin current reason phrases.
- Keep five central checker implementations because the checks recur across consumers; do not copy scanner code to sibling repositories.
- Keep E/F human-owned because choosing a product directory is a domain decision not derivable from disk.
- Keep the valid `.slnx` files already on disk and validate their contents; do not delete them.
- Do not archive this change. The user handover explicitly requires authorization before archive; completion is represented by the two commits and HANDOFF evidence.
