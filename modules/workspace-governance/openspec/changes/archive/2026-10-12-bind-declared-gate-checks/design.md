# Design: Binding a declared gate check to a command derived from disk

## 1. The shape of the defect

A gate manifest has two halves that must agree:

```yaml
checks:                 # what the repository says it will run
  tests: true
commands:               # how each selected check is actually run
  tests: 'cargo test --workspace'
```

`checks` without `commands` is a declaration the Gate cannot execute. It
reports `UNBOUND` and fails the project. Across the fleet, 283 of 421
required checks are in that state, in 69 of 76 manifests.

`bind_verification_command.py` already closed the same gap for the one
concern whose command every repository declares for itself
(`verification.command` in `.project.json`). It cannot close this one,
because a repository's *declared verification command* is not a
per-concern command: `kairosa`'s is
`openspec validate --all --strict --no-interactive`, which is not a test
command, and 40 repositories' would be wrong for `tests` for the same
reason. Those commands are already bound. This change is about the other
four concerns.

## 2. Why detection cannot read `stack`

`.project.json` `stack` is a human-facing label. Measured against disk
it is wrong in shape often enough to make it useless as an input:

- `linglitch` declares `typescript` and its Node manifest is at
  `frontend/package.json`, with a .NET solution at `backend/`.
- `alltools-platform` declares `python` and has no Python at the root.
- `retakeKit` declares `typescript` and has `RetakeKit.sln` at the root.
- `loreMix` declares `typescript` and has `LoreMix.slnx` at the root.
- 19 repositories carry **no `.sln` at all** and use `.slnx` instead.

So the tool reads files and never the label. `stack` is not even passed
into the detection functions, so it cannot leak in later.

## 3. Finding out what the "missing `.sln`" repositories are

The brief's premise was that 8 `dotnet` repositories had no solution at
depth 1 and might be broken. They are not broken, and the reason is a
format change rather than a missing file:

| Shape | Repositories |
|---|---|
| `.sln` only | 26 |
| `.slnx` only | 19 |
| both | 1 (`dotnet-platform-libs` has a root `Platform.sln` plus a fixture `Adopted.sln`) |
| `.csproj` only, no solution | 3 (`aegify`, `hestiaLab`, `smotoox`) |
| neither | everything not `dotnet`-shaped |

Every `.slnx` repository pins `global.json` to SDK `10.0.400`, which
reads `.slnx` natively, so `.slnx` is a first-class solution marker and
not a fallback. The 3 genuinely solution-less repositories are a real
finding: with 2–4 `.csproj` files and no solution, there is no single
`dotnet build` entry point, and the tool says so instead of picking one.

Two repositories have more than one solution. In both, the extra one is
either a test fixture or a vendored sub-project
(`tests/…/Fixtures/adopted/Adopted.sln`,
`git-credential-manager/Git-Credential-Manager.sln`). §4 resolves this
by preferring a root-level solution, not by pattern-matching directory
names, so the rule cannot be defeated by a differently named folder.

## 4. The dominance rule, and the one thing it must never do

Several repositories contain more than one build system: a .NET
solution plus a Node frontend, or a Rust workspace plus Playwright
scripts. A gate has exactly one command slot per concern, so the tool
must pick one, and picking the wrong one is the failure this task
exists to prevent.

The rule is one sentence:

> Collect every candidate entry point that **can answer the concern**;
> if any candidate sits at the repository root, drop the nested ones; if
> exactly one remains, bind it; otherwise the concern is `unresolved`
> and every candidate is named in the reason.

Three properties make it safe:

1. **The filter comes first.** `alltools-platform`'s root
   `package.json` has no `test` script, so it cannot answer `tests` and
   is dropped before dominance is considered. The nested Rust workspace
   is then the only candidate, and it binds. A rule that counted
   manifests before asking whether they could answer would report a
   false ambiguity.
2. **Root beats nested, never the reverse.** `loreMix`'s `LoreMix.slnx`
   is at the root and `web/package.json` is nested, so the solution
   wins. `linglitch` has `backend/Linglitch.slnx` and
   `frontend/package.json`, both nested, so it stays ambiguous.
3. **Ties are reported, not broken.** `retakeKit` has a root solution
   and a root `package.json` with a `test` script. Two root candidates
   is genuinely two answers, so `retakeKit`'s `tests` is `unresolved`
   and `gate-debt.json` names both. There is no tiebreaker to argue
   about and no silent preference for the toolchain that happens to be
   first in a sorted list.

## 5. The derivation table

Every rule below is gated on a file being present. No rule fires on an
absent file, and no rule falls through to a default command.

### Shapes

| Shape | Detected by |
|---|---|
| `dotnet` | `*.sln` or `*.slnx` at any depth, or any `*.csproj` |
| `rust` | `Cargo.toml`, plus whether `[workspace]` appears in it |
| `node` | `package.json`, plus which of its `scripts` keys exist |
| `python` | root `pyproject.toml` / `pytest.ini` / `conftest.py`, or `tests/test_*.py` |
| `flutter` | `pubspec.yaml` |

Directory walk skips `.git`, `node_modules`, `target`, `bin`, `obj`,
`.venv`, `venv`, `dist`, `build`, `.next`, `site-packages`, `.dart_tool`
and the Python/JS caches. This is not cosmetic: `alethefy`'s only
`conftest.py` on disk is
`.venv/lib/python3.12/site-packages/numpy/conftest.py`, and reading it
would make a NumPy vendored file look like the repository's own pytest
configuration.

### Concerns

| Concern | Derivation |
|---|---|
| `repository`, `repository_integrity` | `git diff --check` — requires a git checkout, which is verified by discovery, not assumed |
| `build` / dotnet | `dotnet build <entry>`; `<entry>` = the single root solution, else the single solution, else the single `.csproj`, else unresolved |
| `build` / rust | `cargo build --workspace` when `[workspace]` is present, else `cargo build`; prefixed `cd <dir> &&` when the manifest is nested |
| `build` / node | `npm run build` **only** if `scripts.build` exists |
| `build` / python | **unresolved** — a pure-Python package has no build step |
| `build` / flutter | **unresolved** — `flutter build` needs a target platform the repository does not name |
| `tests` / dotnet | `dotnet test <entry>`; with no solution, only when exactly one test project is detectable |
| `tests` / rust | `cargo test --workspace` / `cargo test` |
| `tests` / node | `npm test` **only** if `scripts.test` exists |
| `tests` / python | `pytest` when a pytest config exists or any test file needs pytest; else `python3 -m unittest discover -s <dir> -v` when every test file is `unittest`; else unresolved |
| `tests` / flutter | `flutter test` |
| `security` / dotnet | `dotnet list <entry> package --vulnerable --include-transitive --no-restore` |
| `security` / rust | `cargo audit` |
| `security` / node | `npm run security` if that script exists, else `npm audit --audit-level=high` |
| `security` / python | `pip-audit` / `bandit -r .` / `safety check` **only** if the repository declares that dependency; else unresolved |
| `security` / flutter | **unresolved** |
| `format` / rust | `cargo fmt --all --check` |
| `format` / node | `npm run format:check` if the script exists |
| `format` / dotnet | `dotnet format <entry> --verify-no-changes` |
| `lint` / rust | `cargo clippy --workspace --all-targets -- -D warnings` |
| `lint` / node | `npm run lint` if the script exists |
| `openspec_names`, `openspec_change_names`, `openspec_name_preflight` | `node scripts/check-openspec-change-names.mjs` when that file exists |
| `product_quality` | `python3 scripts/product_code_quality.py .` when that file exists |
| any other declared concern | `scripts/check-<concern-id>.{sh,mjs,js,py}` when exactly one such file exists; else unresolved |

Two entries in that table are deliberately conservative and are worth
naming:

- **`security` for Node defaults to `npm audit`.** `npm audit` needs no
  script key to exist and is the dependency audit the concern means. It
  will exit non-zero on a real advisory, so binding it can turn a
  repository red. That is the correct outcome for a security check that
  was previously `UNBOUND`; the alternative is a check that passes
  because it never ran.
- **`security` for Python requires the declared dependency.** Unlike
  `npm audit`, there is no stdlib Python dependency audit, so the tool
  binds one only when the repository's own `pyproject.toml` or
  `requirements*.txt` names it. `alethefy` and `ariadex` declare
  `pip-audit`; `jenkins-local` and `workspace-governance` declare
  nothing and stay unresolved rather than acquiring a command nobody
  installed.

### The `tests`-per-Python rule is content detection, not convention

A python repository's test files are read, because the two runners are
not interchangeable:

| Repository | test files | runner bound |
|---|---|---|
| `metiscope` | 55 with bare `def test_`, `[tool.pytest.ini_options]` | `pytest` |
| `sisyphusfy` | 54 with bare `def test_` | `pytest` |
| `workspace-governance` | 24, all `unittest` | `python3 -m unittest discover -s tests -v` |
| `ariadex` | 70, all `unittest` | `python3 -m unittest discover -s tests -v` |
| `platform-contracts` | 3 `unittest` + 3 bare | `pytest` |

The mixed row is the reason the rule reads content. `unittest
discovery` finds no `TestCase` in a bare-function file and reports
success having run nothing — a green check that tested nothing, which
this workspace treats as worse than no checker. `pytest` collects both
kinds, so a repository that needs pytest anywhere gets pytest.

The reverse mistake is avoided too: `workspace-governance` is not bound
to `pytest`, because nothing in its 24 test files needs pytest and
`pytest` is not one of its declared dependencies.

## 6. Reuse, not a second YAML editor

The sibling `bind_verification_command.py` already contains a
comment-preserving YAML editor: block-bounds detection, entry reading,
and single-quoted scalar rendering. Quoting is not a style question — a
double-quoted scalar would re-interpret `\\s` and `$` in a command like
`rg -e 'password\s*'`, and a plain scalar would break on the `:` and `#`
that commands contain.

Two tools that each wrote their own editor would be two places for that
to be wrong. So the two private helpers become public
(`top_level_block_bounds`, `entry_value`) and this tool imports them
along with `yaml_scalar`, `discover_repos` and `is_dirty`. That is a
six-line rename in the sibling and no behaviour change; its tests do not
import the private names, and its 19 tests are re-run to prove it.

## 7. Structure, matching the sibling

```
ConcernOutcome  repo, concern, status, command, evidence, reason
Outcome         repo, status, detail, wrote, before, after, concerns
Report          outcomes, count(), names(), unresolved()
```

`Outcome` keeps the sibling's field names and adds one, so a caller
that reads `repo`/`status`/`detail`/`wrote`/`before`/`after` reads the
same thing here. `ConcernOutcome` is new because a repository's unit of
work here is a concern, not a repository.

Per-concern status:

| Status | Meaning |
|---|---|
| `BOUND` | derived, and written (or would be, under `--apply`) |
| `ALREADY_BOUND` | a command is present and identical to the derived one |
| `CONFLICT` | a command is present and differs — reported, never overwritten |
| `UNRESOLVED` | no command could be derived; `reason` says why |

Per-repository status is the roll-up a reader needs first:

- `RESOLVED` — every `checks: <id>: true` has a command
- `PARTIAL` — some do, some do not
- `UNRESOLVED` — none of the required ones do
- `NO_MANIFEST`, `DIRTY`, `REFUSED` — as in the sibling

A `CONFLICT` counts as not-resolved, because the outcome this tool
produced for that concern is nothing.

## 8. The debt record

`gate-debt.json` at the governance root, beside
`rules-drift-baseline.json` — a committed data file, written on
`--apply`, in the same shape of decision as that baseline.

```json
{
  "generator": "scripts/bind_gate_commands.py",
  "totals": { "repositories_with_debt": 0, "unresolved_checks": 0 },
  "by_reason": [ { "reason": "...", "checks": 0, "repositories": [] } ],
  "repositories": [ { "repo": "...", "shape": "...", "unresolved": [ { "concern": "...", "reason": "..." } ] } ]
}
```

Three deliberate properties:

- **No timestamp.** A `generated_at` field makes every run a diff and
  trains reviewers to ignore the file. Re-running on an unchanged fleet
  produces a byte-identical record, so a change in it is a real change
  in the debt.
- **`by_reason` first.** A person opening the file sees "38 checks:
  no single .NET entry point" before they see a repository name, which
  is the level at which the fix is decided.
- **Sorted everywhere.** Repository and reason order is alphabetical, so
  a re-run diff is a real delta.

## 9. Exit codes

The sibling returns 0 on a completed run and 2 for an unusable root.
That is kept. A third code is added, and it is the point of the change:

| Code | Meaning |
|---|---|
| 0 | every declared check in every repository has a command |
| 2 | the workspace root is not a directory |
| 3 | the run completed and debt remains |

Exit 3 is deliberate and has no escape flag. A wiring tool that exits 0
while 283 checks are unbound is the same defect as a gate that reports
`UNBOUND` and passes: the finding is produced and then discarded. The
debt is meant to be impossible to ignore, so the only way to reach 0 is
to resolve it. This is the one place this tool deliberately diverges
from its sibling, and the sibling's behaviour is not a counter-argument:
it has no debt concept, because a repository it refuses is a repository
whose single declared command was already bound by an earlier tool.

## 10. What is deliberately unchanged

- **No sibling manifest is written by this change.** The verification
  required by this task is a dry run across all 81 repositories. Applying
  the 283 bindings to 69 sibling repositories is a separate
  authorization under the cross-repo boundary in `AGENTS.md`, and the
  `--apply` flag exists for it without this change exercising it.
- **`declared-verification` is untouched.** It is bound by the sibling
  generator from a different source, and this tool reports it
  `ALREADY_BOUND` like any other already-correct concern.
- **No `checks:` entry is added, changed or removed.** A concern is only
  ever bound if the manifest already selected it, which is what keeps
  this tool from turning a repository's gate into a policy it never
  asked for.
- **The fleet audit is untouched.** No `.project.json` and no
  `projects.json` is written, so `errors` and `warnings` do not move.
- **No `rm`.** Scratch and cleanup use `tempfile.TemporaryDirectory()`
  and `shutil.rmtree(..., ignore_errors=True)`, as the sibling does.

## 11. Measurement

| Measurement | Before | After |
|---|---|---|
| Repositories fully resolved | 7 | `tasks.md` §1.1 |
| Repositories partially resolved | — | `tasks.md` §1.1 |
| Repositories fully unresolved | — | `tasks.md` §1.1 |
| Unresolved checks recorded in `gate-debt.json` | — | `tasks.md` §1.1 |
| Test suite | 615 | `tasks.md` §3.4 |
| `openspec validate --changes --strict` | pass | pass |

## 12. Paying the debt as work starts, rather than in a cleanup push

§8 writes the record. Nothing read it, so 22 repositories could carry an
unbound required check indefinitely and start the next feature the same
afternoon. The record is the honest description of the debt; the precondition
below is what makes it shrink on its own.

### 12.1 Why the rule lives in the checker and not in `.ai-rules`

`completion.md` is prose an agent reads before it works. A document does not
block anything: it is read when read, and a skipped document is silent. The
only surface in this repository that a run can fail on is
`scripts/workspace_check.py`, which already emits `RULES_DRIFT_NEW` against a
committed ratchet and runs inside the `governance-quality` gate. So the
precondition is a finding there, and `completion.md` gains one line naming it.

The rule is **fail-closed, and grandfathered exactly once**. The three earlier
drift rules (`TAXONOMY_AUDIT_ENABLED`, `SPEC_PURPOSE_AUDIT_ENABLED`,
`VERIFICATION_BINDING_AUDIT_ENABLED`) ship `False` because each would otherwise
be red for a reason the fleet has not accepted yet. A precondition has the
opposite shape: it is *supposed* to be red while debt is open, and a flag that
turns it off is the switch a rule must not have.

What it must not do is fail the work that predates it. The first cut keyed on
"has any unarchived change" and turned all 22 debted repositories red on
arrival — `jenkins-local` on 8 open changes, this repository on 8 of its own,
including its own `--self-check`. That is a deadlock, not a ratchet: an open
change can only be closed by finishing it, so the rule forbade the only action
that could satisfy it, and the fleet could never come back. The trigger is
therefore *new* work, and the work that was already open when the rule landed is
recorded once in `gate-debt-baseline.json` (§12.4), generated from disk.

That is a committed list, not a switch: no environment variable, no flag, no
in-memory override, and a reviewer can read every entry. The same argument
applies to the exception record. What it is not is an exemption with a life of
its own — a repository that discharges its debt leaves `gate-debt.json` and is
never consulted again, so a stale baseline entry is dead weight rather than a
permanent hole.

### 12.2 The trigger, read from the record rather than re-derived

Both halves of the trigger are read, not recomputed:

- the unbound half comes from `gate-debt.json`, which `GateOracleTest` pins
  **set-equal** to `driftwatchdog gate --dry-run`'s own unbound set. A second
  parser over manifests would be a second source of truth for the same fact,
  and would drift the first time a profile scheduled a check `checks:` never
  listed — which is the bug §5.1 already found once;
- the new-work half comes from the repository's own `openspec/changes`
  directories, which `validate_change_names` already walks. The new rule reads
  the same helper, so the two cannot count different sets.

### 12.3 The debt-closing change is not a trigger

`close-unbound-gate-checks` is excluded from the trigger set. Without that
exclusion the rule forbids the only work that can discharge the debt, and every
repository with debt becomes permanently red — a dead end that teaches agents
to disable the rule.

### 12.4 The two committed records, in the shape of the drift baseline

The codebase already has two answers to "how does an owner record that this
is deliberate", and this rule uses the one that is evidence:

- `discovery.unregistered_policy` in `projects.json` is a **policy** — a
  fleet-wide default in a machine-read registry. It suits a choice about
  discovery, which is a portfolio-wide decision;
- `rules-drift-baseline.json` is a **ratchet** — a committed list of
  `(project, file, label)` keys a reviewer can read and tighten. It suits a
  choice about one repository's finding.

A deliberate exception to a per-repository precondition is the second kind, so
it is `gate-debt-exceptions.json`: a list of `(project, change, reason)` in
the governance root, loaded with the same fail-closed refusals as the drift
baseline, an unknown key refused rather than ignored, and no environment
variable and no flag.

The grandfathering list is the same kind of thing one step earlier in time, and
so is the same shape: `gate-debt-baseline.json`, a list of `(project, change)`
with no `reason` because the reason for each is "it was already open".
`scripts/gate_debt_baseline.py` writes it from `gate-debt.json` and the
repositories' own `openspec/changes` directories, through the same two helpers
the rule reads, so the ratchet and the rule cannot disagree. It also refuses a
root that holds none of the debted repositories, because a mistyped root would
write an empty ratchet and an empty ratchet grandfathers nothing — the whole
fleet red again, from a typo. `--check` reports drift and writes nothing; the
checker never writes these files at all.

A satisfied pair is reported at `WARNING`, not dropped. The reason is quoted
and the record is named, so the exception is visible in the same run's output
that a reader of the change would want to see, and the finding stays
reviewable as the fleet shrinks. Only `ERROR` moves the exit code.

The three records differ in exactly one respect, and it is deliberate: the
drift baseline requires a non-empty `findings` array because an empty one would
mean every known drift had been fixed, which is a claim worth making loudly.
An empty ratchet here means only that no debted repository has opened a change
since the last snapshot, and an empty exception list is the normal state, so
both accept `[]`.

### 12.5 The one cross-repo step, and why it is not optional

`GOVERNANCE_QUALITY_DECLARED_CODES` is a closed allow-list: the concern drops
any finding whose code is not in it *before* computing the status, and treats
the checker's exit 1 as an expected exit. An undeclared code therefore yields
`PASS` while the checker exits 1 — green gate, red checker, no finding
anywhere. That is the trap this workspace has already fallen into once
(`HANDOFF.md` on `spec-purpose-backfill-gate`). The new code is declared there
in the same change, and the acceptance test is a real `driftwatchdog gate` run
over a fixture, not a unit test of the list.
