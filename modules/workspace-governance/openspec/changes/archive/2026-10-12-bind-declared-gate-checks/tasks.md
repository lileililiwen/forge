# Tasks: bind-declared-gate-checks

## 0. Caveats and constraints

- [x] **The declared `stack` is not an input.** `.project.json` `stack` is
  wrong for 27 of the 81 repositories by shape, so it is not passed into
  any detection function (`design.md` §2).
- [x] **No sibling repository is written.** The verification this change
  requires is a dry run across all 81 repositories. Applying the bindings
  to 69 sibling manifests is a separate authorization under the
  cross-repo boundary in `AGENTS.md`; `--apply` exists for it and this
  change does not exercise it over the fleet (`design.md` §10). It is
  exercised over `tmp_path` fixtures only.
- [x] **No concern is added or removed.** A concern is bound only if the
  repository's own `checks:` already selected it.
- [x] **A gap is recorded, never closed with a guess.** A wrong command
  is worse than an honest gap.
- [x] **No `rm`.** Cleanup uses `tempfile.mkdtemp()` plus
  `shutil.rmtree(..., ignore_errors=True)`.
- [x] **No commit sweeps unrelated work.** The tree carried an untracked
  `openspec/changes/bind-declared-verification/` directory from an
  earlier session. It is not staged and not committed by this change.
  **Resolved after this change landed:** that directory held only the
  `--apply` run's rollback record and no change package, and the change
  itself is `bind-declared-verification-command` in `driftwatchdog`,
  which owns the Gate contract these spec deltas amend. The record is
  now tracked at
  `migration/bind-declared-verification-command-rollback.json` and the
  empty directory is gone, so `openspec validate --all --strict` is
  22/0. `main` only, no branch, no `git add -A`, no attribution
  trailers, no push.

## 0b. Where the previous session stopped

The generator, the sibling rename and this spec directory were written. The
work was aborted before the tests and before either commit. This section
records the state the tests were added to.

- [x] The generator `scripts/bind_gate_commands.py` is complete and works.
  A fleet dry run over all 81 repositories resolves 51 and partially
  resolves 25, with 0 refused, 48 unresolved checks, 18 left in place and 5
  repositories with no manifest. Measured before the tests were written
  and again after, unchanged.
- [x] `scripts/bind_verification_command.py` differs from `HEAD` only by
  the two-helper rename (`_top_level_block_bounds` →
  `top_level_block_bounds`, `_entry_value` → `entry_value`) with every
  call site updated, so the new generator can reuse the existing YAML
  editor. The sibling's own tests pass and its own dry run still reports
  `BOUND: 0, ALREADY_BOUND: 76, CONFLICT: 0, REFUSED: 0`, so the rename
  changed no behaviour.
- [x] **The generator had no tests at all.** That is the gap this pass
  closes. `tests/test_bind_gate_commands.py` now holds 92 tests.

## 1. BFS — Baseline and blast radius

- [x] 1.1 Record the baseline before any edit: suite `Ran 615 tests … OK`;
  76 manifests; 421 required checks; 138 bound; **283 unbound across 69
  repositories**; 7 fully wired; 19 repositories on `.slnx` and 26 on
  `.sln`.
- [x] 1.2 Settle the "8 dotnet repositories with no `.sln`" premise by
  inspection rather than assumption: they are not broken. 19 repositories
  use `.slnx` and pin SDK `10.0.400`; the remaining 3 (`aegify`,
  `hestiaLab`, `smotoox`) genuinely have 2–4 `.csproj` files and no
  solution, which is a finding and not a search failure.
- [x] 1.3 Map every consumer of the sibling's YAML editor before reusing
  it: `bind_verification_command.py` is imported by
  `measure_bound_verification.py` and by
  `tests/test_bind_verification_command.py`, and neither imports the two
  private helpers.
- [x] 1.4 Record the awkward cases that must be handled by disk, not by
  label: `linglitch` (`backend/Linglitch.slnx` plus
  `frontend/package.json`), `loreMix` (root `LoreMix.slnx` plus nested
  `web/package.json`), `alltools-platform` (declared `python`; root Node
  with no `test` script, Rust workspace under `rust/`), `retakeKit`
  (root solution **and** root `package.json` with `test`).

## 2. DFS — One coherent contract

- [x] 2.1 `detect_shape(repo)` walks the disk once and returns the
  solutions, project files, crates, node manifests and their script keys,
  Python runner evidence, Flutter manifests and declared Python security
  tooling, skipping every vendored and cache directory in
  `design.md` §5.
- [x] 2.2 `candidates(concern, shape)` returns the entry points that can
  answer the concern, each with its evidence path, applying the
  dominance rule: filter by capability, prefer root-level, and refuse a
  tie (`design.md` §4).
- [x] 2.3 `derive(concern, shape)` maps a single surviving candidate to a
  command. Every rule is gated on a file existing; there is no default
  and no fallback.
- [x] 2.4 Concerns with no shape rule fall through to two file-gated
  derivations — `scripts/check-<concern-id>.*` and the named OpenSpec-name
  and product-quality scripts — and are otherwise `unresolved`.
- [x] 2.5 `ConcernOutcome` and `Outcome` keep the sibling's field names,
  with `Outcome` gaining only `concerns`, and `Report` gains
  `unresolved()` for the roll-up.
- [x] 2.6 Reuse the sibling's YAML editor rather than writing a second
  one: make `top_level_block_bounds` and `entry_value` public and import
  them with `yaml_scalar`, `discover_repos` and `is_dirty`.
- [x] 2.7 `render_manifest` inserts only the concerns that are missing, in
  the manifest's own `checks:` order, and writes the generated comment
  once. *Correction from the original wording:* `render_manifest` is **not**
  idempotent on its own, and should not be. Idempotence is a property of
  the run: the second run classifies the already-bound concern as
  `ALREADY_BOUND`, so there is no binding left to insert and the file is
  never rewritten. The tests pin the run, not the renderer.
- [x] 2.8 `write_debt_record` emits `gate-debt.json` with `totals`,
  `by_reason` and `repositories`, sorted, with no timestamp.
- [x] 2.9 Exit codes: 0 clean, 2 unusable root, 3 debt remains, with no
  flag that suppresses 3 (`design.md` §9).

## 3. BFS — Re-audit every consumer

- [x] 3.1 Pin the contract with tests covering: the label is not read; a
  `.slnx` is a solution; a vendored `conftest.py` is not evidence; the
  workspace flag follows `[workspace]`; a nested manifest carries its
  `cd`; the exact `package.json` script key is used; a missing `test`
  script produces no command; two candidates stay unresolved; a
  root-level candidate wins; a candidate that cannot answer is dropped
  first; a pure-Python `build` is unresolved; a mixed unittest/pytest
  repository binds `pytest`. → `DetectionTests` (12), `DerivationTests`
  (19), `RefusalTests` (11), `AmbiguityTests` (6).
- [x] 3.2 Pin the write contract: dry run writes nothing; `--apply`
  writes; a second run is byte-identical; a differing existing command is
  a conflict and is not overwritten; an undeclared concern is never bound;
  a `false` check stays `false`; a dirty manifest is skipped; the
  generated comment is not duplicated. → `WriteContractTests` (13),
  `GitWriteContractTests` (3), `WorkspaceWriteTests` (4).
- [x] 3.3 Pin the debt record: every unresolved concern appears with its
  reason, `by_reason` totals match the check count, the file is sorted
  everywhere, two dry runs over an unchanged fleet are byte-identical, and
  there is no timestamp. → `DebtRecordTests` (10).
- [x] 3.4 Suite green: `707 passed, 158 subtests passed`, up from
  `615 passed, 155 subtests passed`. 92 tests added; none weakened, none
  skipped, none filtered to reach green.
- [x] 3.5 Negative control, run and then reverted. Making `derive` return
  `python3 -m build` for a pure-Python package turned three tests red:
  `test_a_pure_python_build_is_unresolved` with
  `AssertionError: 'BOUND' != 'UNRESOLVED'`,
  `test_a_pure_python_repository_never_acquires_a_build_command` with
  `AssertionError: ' -m build' unexpectedly found in 'python3 -m build'`,
  and `test_the_verbose_line_names_an_unresolved_reason`. The binder was
  restored byte-identical and the suite returned to 707.
- [x] 3.6 Fleet dry run across all 81 repositories, with per-repository
  evidence for at least 6 repositories across different stacks and the
  three awkward cases named in §1.4. → `FleetTest` (6) walks the real
  workspace once and asserts invariants only, never a count, so it
  survives fleet churn: no repository is refused; every path a bound
  command or its evidence names exists in that repository; at least one
  `.slnx` repository still receives a .NET command; no bound `dotnet`
  command omits its entry file; every unresolved concern carries a reason;
  every bound concern carries a command and its evidence.
- [x] 3.7 `README.md` documents the generator, debt record, dry-run/apply
  modes, repeatable `--repo`, `--verbose`, and exit codes.

## 4. BFS — Spec and validation

- [x] 4.1 Spec delta at `specs/gate-command-binding/spec.md`, a new
  capability: the fleet has no gate-command-binding spec, and the
  existing `rules-baseline` capability governs the rules text, not gate
  commands.
- [x] 4.2 `openspec validate bind-declared-gate-checks --strict` passes.
- [x] 4.3 `openspec validate --changes --strict` passes for the whole
  tree. (`openspec list --changes --strict` is not valid on the pinned
  CLI v1.6.0; `openspec list --changes` is the available form.)
- [x] 4.4 Two commits on `main`: spec, implementation, tests and the
  sibling rename first, then `HANDOFF.md` only. No archive commit, no
  branch, no push.

## 5. The debt record under-reported: the defect this pass found

- [x] 5.1 **Measured, not assumed.** The gate (`driftwatchdog gate
  --dry-run`, the build at `driftwatchdog/target/debug/`) reports **55
  repositories and 170 unbound required checks**. The committed
  `gate-debt.json` named **25 repositories and 48 checks**. 122 pairs
  across 30 repositories were missing, and no pair was spurious. The
  `driftwatchdog` on `PATH` is a `cargo install` build from 2026-10-01
  that predates unbound reporting and answers 0 for the whole fleet; the
  scan must use the repository build.
- [x] 5.2 **The cause, with the evidence.** A manifest selects a check two
  ways: by listing it under `checks:`, and by naming a `profile:` whose
  default set the gate schedules anyway (`driftwatchdog/src/gate/manifest.rs`
  `selected_without_triggers` → `profile_defaults`). `read_manifest` read
  only the first. Every missing pair is a concern the gate's
  `product`/`rust-product` profile schedules: 41 `placeholder-threshold`,
  41 `product-code-boundary`, 40 `governance-quality` — the 41st is
  `driftwatchdog`, the one repository that lists `governance-quality` under
  `checks:` itself and is therefore the single `governance-quality` entry
  the old record did have. `aesthra` is the shape: six declared checks, all
  bound, three profile-scheduled checks the gate still fails on, and no
  entry in the record at all. A repository with no unresolved check of its
  own was dropped for "having no gaps", so the record read as complete.
- [x] 5.3 **The stated hypothesis was wrong and is not what was fixed.** The
  binder did report the no-rule/no-fallback class: `script_concern()`
  returns exactly the reason string quoted in the bug report, and that
  string is in `by_reason` for the one repository that declares
  `governance-quality` under `checks:`. Nothing was silently skipped for
  want of a rule; the class was invisible because those repositories never
  declared the concern in the first place.
- [x] 5.4 **The fix.** `PROFILE_DEFAULTS` mirrors the gate's
  `profile_defaults`, and `gate_required()` resolves the set the gate would
  fail on: the manifest's own `checks:`, plus the profile's defaults, minus
  what the gate excuses — a `checks: <id>: false` that deselects a profile
  default, a `not_applicable:` exemption with a reason, and
  `source-file-size`, which the gate executes in-process
  (`is_source_file_size_concern`) and never counts unbound. `"optional"`
  stays out because `read_manifest` reads only `true`, and the gate does
  not fail on a non-required check.
- [x] 5.5 **A profile-scheduled check is recorded, never written.** Proved
  against the binary: a manifest whose `commands:` names a check the
  manifest does not select is refused outright —
  `manifest parse error: commands.governance-quality references a check
  that is not selected`. Writing one would trade a reported gap for a
  manifest the gate cannot parse, so the reason tells the reader the fix
  instead: select it under `checks:`, add `scripts/check-<id>.*`, or
  exempt it.
- [x] 5.6 **The second, quieter hole: a dirty manifest.** The `is_dirty`
  check ran before the manifest was read, so a repository with uncommitted
  work returned with no concerns and vanished from the record while the gate
  still failed on it — the exact behaviour §5 below recorded as a
  reporting quirk. The write is still refused; the classification is not.
  An `--apply` run immediately after a previous one no longer writes a
  record that omits every repository it had just written to.
- [x] 5.7 **The record is regenerated and committed.**
  `python3 scripts/bind_gate_commands.py --root /home/paul/code --apply
  --debt-out /home/paul/code/workspace-governance/gate-debt.json` — run
  from a clean fleet, so the record is not built from a run that skipped its
  own writes. It now names **55 repositories and 170 checks**, equal to the
  gate's own count. A dry run first confirmed `0` repositories with a
  pending binding, so `--apply` wrote **no** sibling manifest; the fleet was
  re-checked afterwards and every `git status --porcelain -- .ai-gate/gate.yaml`
  across the 81 repositories is empty.
- [x] 5.8 **The regression is now impossible to reintroduce silently.**
  `ProfileSelectionTests` (12) pins the selection model on fixtures,
  including the repository whose only debt is profile-scheduled, the
  in-process check, the disabled check, the exemption, the optional check,
  and the refusal to write. `GateOracleTest` (5) compares the committed
  record **and a fresh dry run** against the real binary's unbound set, by
  set equality and never by count, so the fleet may grow or re-profile
  freely. It skips when the binary is not built, and skips with a named
  reason when the binary is too old to report unbound checks — proven first
  against a throwaway one-check repository, so the comparison can never run
  against a blind oracle.
- [x] 5.9 **Negative controls, run and then reverted.**
  (a) Making `gate_required` return the manifest's `checks:` alone — the
  pre-fix behaviour — turned 5 tests red, including
  `test_a_repository_whose_only_debt_is_profile_scheduled_is_still_listed`
  with `AssertionError: demo is missing from the debt record; the record
  names []` and `test_a_fresh_run_reproduces_the_gate_set` with the 122-pair
  diff and the message "the gate reports unbound checks a fresh run does not
  find; the record would be regenerated without them".
  (b) Moving the `is_dirty` check back above the classification turned
  `test_a_dirty_manifest_is_still_classified_and_recorded` red with
  `AssertionError: Lists differ: [] != [('demo', ['build', 'tests'])]`.
  Both controls were reverted and the suite returned to 722.
- [x] 5.10 **The shared fixture moved to `profile: minimal`.** It said
  `profile: product`, so every one of the 89 fixture tests gained three
  unresolvable concerns that had nothing to do with what each test was
  about. `minimal` is the built-in profile with an empty default set, which
  leaves `checks:` as the only selection in play and each test asserting
  exactly what it asserted before. The profile path is covered against
  `product` itself in `ProfileSelectionTests`. No assertion was weakened,
  skipped or filtered.
- [x] 5.11 Spec delta extended: a new requirement, *The debt record is
  complete against the gate's own unbound set*, with five scenarios, and
  *Only a concern the repository declared is bound* amended to say that a
  `profile:` is a declaration and that a profile-only check MUST NOT be
  written.

## 6. Re-audit the recorded follow-ups

- [x] Re-measure the current fleet without writing sibling manifests:
  `bind_gate_commands.py --root /home/paul/code` reports 82 repositories,
  0 unresolved checks, and leaves 29 existing commands in place; the legacy
  debt driver reports 0 entries. No fleet `--apply` was run for this archive.
- [x] Keep the generator fail-closed for `.csproj`-only shapes without one
  derivable entry point. The recorded live commands are preserved; no
  solution was synthesized to make the report green.
- [x] Resolve the former `retakeKit` / `linglitch` multi-stack ambiguity in
  the separately archived `derive-composite-gate-command` change. The
  generator now composes disjoint stack candidates and preserves refusal for
  same-stack ambiguity.
- [x] Keep `.project.json` `stack` out of detection. The separately archived
  identity-and-taxonomy migration resolved the portfolio axes, and
  `axis_migration.py --check` reports no outstanding rows.
- [x] Fix the Python shape-reporting gap: a `pyproject.toml` now identifies a
  Python shape even without a test directory. A regression test first failed
  with `unknown`, then passed after the fix. The apply/dirty-manifest
  idempotency correction remains covered by its existing tests. The focused
  binder suite passes 139 tests with 1 skipped.
- [x] Re-run `FleetTest`: all 6 fleet invariants pass, including path
  existence and `.slnx` coverage.
- [x] Archive this completed change with the owner's authorization. Remaining
  ambiguous source shapes and unrelated owner decisions stay visible as
  documented boundaries, not guessed bindings.

## 6. The `quality` block is derived, so 80 more checks can bind

`placeholder-threshold` and `product-code-boundary` were 80 unbound checks in
40 repositories: both are answered by `product_code_quality.py`, which is
fail-closed and exits 2 without a `quality` block in the repository's own
`.project.json`. Only 2 of 81 repositories declared one. The previous pass
refused to write one, on the reasoning that the roots, languages and threshold
are all owner decisions. That conflated two different things.

- [x] 6.1 **Two of the three fields are derivable.** `product_roots` and
  `languages` are facts about where the code is, so `derive_quality_block`
  derives them from the same disk detection the build commands use, and
  attaches the manifest each root came from as evidence. The third,
  `placeholder_threshold`, is a judgement and is written as `0` — the
  deliberately strict default both hand-written blocks chose, documented as
  relaxable rather than neutral.
- [x] 6.2 **No `exclude` and no marker list are invented.** A guessed
  exclusion hides a real finding instead of reporting one, and the marker
  vocabulary is the checker's own (`vocabulary.json`), so a copy in 40
  declarations would be a second source of truth that drifts.
- [x] 6.3 **The `src` convention is per manifest kind.** A Rust crate and a
  Node package keep sources in `src/`; a .NET project keeps them **beside the
  `.csproj`**. Getting this wrong refused 22 of the 40 repositories, which is
  what the first implementation did.
- [x] 6.4 **The block and the command are written in one operation**, both
  under `--apply` and both behind the dirty check — `.project.json` now gets
  the same uncommitted-work guard the manifest had. Until the block is on
  disk the concern stays debt, because a `commands:` entry whose checker
  still exits 2 is a bound check that runs nothing.
- [x] 6.5 **The debt record and the gate still agree exactly**: 125 = 125
  across 53 repositories, confirmed by set equality against
  `driftwatchdog/target/debug/driftwatchdog`, not by count. It fell from 127
  because `driftwatchdog` is the one repository the one-repository proof was
  authorised against.
- [x] 6.6 **Negative control.** Removing the `source.is_dir()` and
  `_language_files_present` guards makes the tool write a `product_roots`
  entry for a directory that does not exist; the checker then reports
  `REVIEW_REQUIRED` / `product_roots entry is not a directory: src` and
  exits 2, and `test_a_repository_with_no_derivable_product_root_is_refused`
  goes red. Both guards restored, 741 green.

### 6b. The fleet write is NOT done, and the red count is why

Proven end to end on `driftwatchdog` alone: `placeholder-threshold` and
`product-code-boundary` moved from `UNBOUND` to **running**, and the gate
reports a real verdict — `FAIL: placeholder marker count 30 exceeds threshold
0`. That is the mechanism working, not a defect.

Applying it to all 40 was measured before it was written, by running the real
checker over each repository with its derived block passed as `--policy`:

| Measurement | Value |
|---|---|
| Repositories the block is derivable for | 40 of 40 |
| Would go **red** under `placeholder_threshold: 0` | **29** |
| Would stay green | 11 |
| Refused (no derivable root) | 0 |

**29 of 40 is not a number to ship quietly.** The breakdown says it is not
what the threshold is for either: of 498 findings, 338 (68%) are the word
`placeholder` and 120 (24%) are `stub` — ordinary vocabulary in prose and in
identifiers — and only 40 are `todo`/`fixme`/`xxx`/`hack`. Excluding
`placeholder` and `stub` leaves **8** repositories red. The strict default is
right for a check that is genuinely about unfinished work; the marker
vocabulary is currently about English words. Narrowing the vocabulary is a
change to `vocabulary.json` and belongs to whoever owns it — it is not this
change's to make, and it is not an `exclude` this tool may invent.

So the fleet write is deliberately **not** applied. The derivation, the one-
repository proof and the measurement are the deliverable; applying it to 39
more repositories is a separate authorisation, taken after the marker
vocabulary is settled.

> **Superseded by §7.** The vocabulary was settled — `placeholder` and `stub`
> dropped — and the write is applied to the 39. With the corrected vocabulary
> the red count is 13 of 39, not 29 of 40, and the debt is 47 checks across 24
> repositories, down from 125 across 53. The measurement above stands as the
> record of why the write was withheld; §7 is the record of what it cost.

## Follow-up: three fleet assertions were vacuous after the fleet was applied

The three `FleetTest` assertions filtered concerns on `status == BOUND`, which
means "this run would have wired it". Once `--apply` had been run across the
fleet, every concern became `ALREADY_BOUND` and the assertions had almost
nothing left to check. They read the same invariant against the committed
commands instead, which is more of the fleet and holds whether or not anyone
has applied the bindings yet.

The `dotnet` assertion also encoded a wrong premise. It required a `dotnet`
command to name a build file, but `dotnet test tests/Foo.UnitTests` names a
project *directory* (no file extension), and a bare `dotnet test` is valid
when the repository root holds exactly one solution or project for it to
resolve. The assertion now accepts a named target, a named project directory,
or exactly one build entry point at the root -- and still fails for a bare
`dotnet build` in a repository that has none.

Verified by negative control: forcing every derived `dotnet build` to name a
non-existent file turns 5 tests red, and forcing a bare `dotnet build` turns
this one red with `aegify/build: dotnet build names no build file, no project
directory, and the root holds 0 build entry points rather than exactly one`.

## 7. The marker vocabulary is corrected, and the fleet write is done

The ruling on the vocabulary was made outside this change: **drop the bare
words `placeholder` and `stub`** from `vocabulary.json`, keep `todo`, `fixme`,
`xxx`, `hack`, `unimplemented!`, `todo!`. §6b's measurement said exactly what
was wrong — of 498 findings, 338 were the word `placeholder` and 120 were
`stub`, ordinary vocabulary in prose and in identifiers, against 40 real
`todo`/`fixme`/`xxx`/`hack` — and it refused to make the call because the
vocabulary belongs to the `portfolio-vocabulary` capability. It has been made
here instead, as a spec correction with its reasoning and its numbers in
`specs/portfolio-vocabulary/spec.md`.

- [x] 7.1 **The vocabulary is corrected at its one source.**
  `vocabulary.json` drops `placeholder` and `stub`. `product_code_quality.py`
  resolves its markers from that file at use time (`default_placeholder_markers`
  → `vocabulary.load_vocabulary_for_governance_root`), so no code change was
  needed and none was made: the marker pattern, the severity mapping and the
  threshold comparison are untouched.
- [x] 7.2 **The premise about `forge` did not hold, and was checked rather
  than assumed.** Every `.project.json` in the fleet was grepped for an inline
  `placeholder_markers`. Exactly one carries one: `forge`. Its list is
  `todo, fixme, xxx, hack, unimplemented!, todo!` — already the corrected set,
  with neither `placeholder` nor `stub` in it. So there was no hand-written
  stricter list to protect and no conflict to resolve. It is left alone
  regardless, per the spec: a project that states a stricter vocabulary on
  purpose has made a deliberate local decision, and it is the only one.
- [x] 7.3 **Re-measured before the fleet write, on the same 39.** With the
  corrected vocabulary the authorised set reports **13 red of 39**
  (was 29 of 40). The 13 split by cause, which matters more than the total:
  7 are red on real markers (`alltools-platform` 11, `openlearning` 18,
  `hookit`/`openaccounting`/`orphevia`/`stylify` 2 each, `lexora` 1), 5 are
  red on `product-test-code` alone with zero markers, and `opendockify` is
  `REVIEW_REQUIRED` on a `readme-claim-unscoped` finding in its own README.
  Only the first 7 are marker findings at all.
- [x] 7.4 **The fleet write is applied to exactly the authorised 39.**
  `--apply` wrote 39 `.project.json` and 39 `.ai-gate/gate.yaml`, nothing
  else. Verified by hashing both files in all 81 repositories before and
  after: 78 files changed, 39 repositories, and the set difference against the
  authorised list is empty.
- [x] 7.5 **The debt record was regenerated in a mode that cannot write a
  sibling.** §6's mistake — `--apply` fleet-wide while regenerating the
  record, writing 40 files the change was told not to write — is now
  structurally impossible for this operation: `--debt-only` threads
  `apply=False` into `run()` and persists only the record, and it refuses to
  combine with `--apply` (exit 2). `DebtOnlyTests` (4) pins that it writes the
  record, writes no manifest, says the manifest run is dry, refuses the
  combination, and produces a record identical to a plain dry run's.
  The `--apply` run's default `--debt-out` was `root/gate-debt.json`, which
  put a stray record at the workspace root; it was removed with
  `Path.unlink(missing_ok=True)` and the record regenerated to
  `workspace-governance/gate-debt.json`.
- [x] 7.6 **Debt and the gate are set-equal, before and after.**

  | Measurement | Before | After |
  |---|---|---|
  | Gate unbound checks | 125 | **47** |
  | Gate repositories with an unbound check | 53 | **24** |
  | `gate-debt.json` `unresolved_checks` | 125 | **47** |
  | `gate-debt.json` `repositories_with_debt` | 53 | **24** |
  | Set-equal? | yes | yes |
  | `placeholder-threshold` unbound | 39 | **0** |
  | `product-code-boundary` unbound | 39 | **0** |

  Set equality, never a count, against
  `driftwatchdog/target/debug/driftwatchdog`. The `driftwatchdog` on `PATH` is
  a 2026-10-01 build that predates unbound reporting and answers 0 for the
  whole fleet; it is not an oracle and was not used.
- [x] 7.7 **Confirmed on real gate runs, not only on the checker.** Both
  directions exist in the fleet: `openlearning` reports
  `placeholder-threshold FAIL — placeholder marker count 18 exceeds threshold
  0` with `unbound: ['tests']`, and `pangoman` reports
  `placeholder-threshold PASS — placeholder marker count 0 at or below
  threshold 0` with `unbound: []`. The two checks are executing and producing
  verdicts in both directions, which is the whole point of binding them.
- [x] 7.8 **Tests.** `741 passed, 158 subtests` before;
  **`748 passed, 158 subtests, 0 failed`** after. 7 added: 3 in
  `PlaceholderTests` (the word `placeholder` in prose is not a finding; the
  word in an identifier is not a finding; `TODO` and `FIXME` still are), 4 in
  `DebtOnlyTests`. One existing test asserted the old list and was updated to
  the new contract, as the spec delta now requires:
  `tests/test_vocabulary.py::test_placeholder_markers_unchanged` became
  `test_placeholder_markers_are_unambiguous_debt_tokens`, which pins the
  retained set and asserts `placeholder` and `stub` are absent. No test was
  weakened, skipped, filtered or given an `exclude`.
- [x] 7.9 **Negative control, run and reverted.** Putting `placeholder` and
  `stub` back into `vocabulary.json` and re-measuring the same 39 with their
  own on-disk blocks: marker findings **38 → 468** (`placeholder` 310,
  `stub` 120), repositories reporting `FAIL` **13 → 28**. The file was then
  restored byte-identically (`git diff --stat` shows only the 2 intended
  deletions) and the suite returned to 748.

## 8. Recorded, not fixed — findings for their owners

- [x] **The Rust scan in `product_code_quality.py` was lexical and
  single-file, so it reported 24 false `product-test-code` findings in
  `driftwatchdog`. FIXED here, as a correction to detection rather than as a
  workaround.** `src/gate/governance/tests.rs` is declared `#[cfg(test)] mod
  tests;` from `mod.rs:627`, and the old scanner read each file in isolation, so
  it could not see that the module is compiled only under `cfg(test)` and
  counted its contents as product test code. The Rust branch now decides test
  scope from the crate's module tree: a `.rs` file is test scope when a sibling
  `mod.rs`/`lib.rs`/`main.rs` — or the parent directory's `mod.rs` for a
  directory module — carries `#[cfg(test)]` on the `mod <name>;` declaration
  that reaches the file, including through a `#[path = "…"]` attribute. The
  guard is also tracked per item, so `#[cfg(test)]`, `#[path = …]` and `mod
  tests;` on consecutive lines are one guarded item; that was a second, separate
  cause of the same false positive. Measured over the fleet on the
  `product-test-code` rule, before → after:

  | Repository | Before | After | Cause |
  |---|---|---|---|
  | `driftwatchdog` | 24 | **0** | `src/gate/governance/tests.rs`, declared `#[cfg(test)] mod tests;` |
  | `rust-ecommerce` | 18 | **0** | `src/cdn.rs` (16, `#[cfg(test)]` then `#[allow(…)]` then `mod tests {`), `src/shipment/mod.rs` (1, `#[cfg(test)]` then `#[path = "tests.rs"]`) |
  | `alltools-platform` | 2 | **1** | `typed_loop.rs` cleared (same `#[path]` shape); `test_prompt.rs` kept — a bare `#[test]` in a file no module declares |
  | `argoset`, `crossify`, `fotofy`, `orphevia`, `transitfy` | 2/1/6/1/6 | unchanged | TypeScript `describe(`/`it(` heuristic, not Rust |
  | fleet total | 60 | **17** | 43 findings removed, all of them Rust |

  The other 5 repositories of the 13 in §7.3 that were red on
  `product-test-code` alone with zero markers — `argoset`, `crossify`, `fotofy`,
  `rust-ecommerce`, `transitfy` — split: `rust-ecommerce` is now clean, and the
  other four are red on the **TypeScript** `describe(`/`it(` heuristic, which is
  a separate defect in a language this change does not touch. `driftwatchdog`
  is still red, now for two real reasons it owns: 2 placeholder markers in
  `src/gate/concerns.rs:20` and 8 files over the 1000-line source limit.

  No `exclude` was added, no filename exception list was introduced, and no test
  was weakened: a `tests.rs` with no `#[cfg(test)]` declaration reaching it is
  still reported, which is what
  `RustModuleTreeBoundaryTests::test_the_declaration_is_what_clears_it_not_the_file_name`
  pins. Negative control, run on a **copy** of `src/gate/governance` in a
  temp directory (`driftwatchdog` itself was never written to): with the
  declaration 0 findings, with `#[cfg(test)] mod tests;` removed from the copy
  24 findings return, restored 0 again.

  Tests: `747 passed, 158 subtests, 1 failed` before and after — the one
  failure, `GateOracleTest::test_the_record_names_nothing_the_gate_calls_bound`
  (`('metiscope', 'build')`), is pre-existing on `main` and unrelated to the
  Rust scan. The baseline quoted in §7.8 was `748 passed, 0 failed`; the
  recorded debt and the live gate no longer agree for `metiscope`, so that
  figure no longer holds on this tree. 13 added, all in
  `RustModuleTreeBoundaryTests`.
- [x] Record the remaining `opendockify` README status claim as checker
  coverage (`QUALITY_README_CLAIM_UNSCOPED`), not as a gate-binding defect.
  The project warning and aggregate coverage warning remain visible; no owner
  decision about the README claim is made here.
- [x] The four TypeScript heuristic false positives were corrected in the
  separately archived `anchor-typescript-test-code-match` change; its scanner
  test suite passes and the binding change does not own that scanner.
- [x] Archive authorization is recorded in the owner's current request.

## 9. The record is read: a new change waits on the debt

- [x] **`QUALITY_NEW_CHANGE_WITH_GATE_DEBT` (ERROR) is emitted** by
  `scripts/workspace_check.py` for a repository in `gate-debt.json` that opens
  an unarchived change which is neither `close-unbound-gate-checks` nor a
  grandfathered pair. Both halves are read: debt from `gate-debt.json` (never
  re-derived), new work from `unarchived_changes`, the same walk
  `validate_change_names` uses.
- [x] **The rule has no fence**, unlike the three earlier drift rules, because
  it is *supposed* to be red while debt is open (`design.md` §12.1).
- [x] **The grandfathering ratchet is generated, not hand-written.**
  `scripts/gate_debt_baseline.py` writes `gate-debt-baseline.json` from
  `gate-debt.json` and the repositories' own `openspec/changes`: **39 pairs
  across 22 repositories**. `--check` reports drift and writes nothing.
- [x] **The generator refuses a root that holds none of the debted
  repositories.** A mistyped root would write an empty ratchet, and an empty
  ratchet grandfathers nothing, so the whole legacy fleet would go red from a
  typo in a path.
- [x] **The exit is a committed record**, `gate-debt-exceptions.json`, in the
  shape of `rules-drift-baseline.json`: `(project, change, reason)`, no
  environment variable, no flag, unknown key refused rather than ignored. A
  satisfied pair is reported at `WARNING` with its reason quoted, so a
  deliberate decision stays visible in the run.
- [x] **All three records fail closed.** A missing, corrupt or mistyped record
  is one run-level `ERROR` naming the file; a ratchet that could not be read
  grandfathers nothing, because a baseline that was never read is not one.
- [x] **`QUALITY_NEW_CHANGE_WITH_GATE_DEBT` is declared** in
  `driftwatchdog`'s `GOVERNANCE_QUALITY_DECLARED_CODES`
  (`src/gate/governance/mod.rs`), with one Rust test beside it. A cross-repo
  step in a repository this change does not own, and measured rather than
  assumed: the false green was reproduced first (§9.1).
- [x] **`.ai-rules/completion.md` gains one line** naming the precondition and
  pointing at the checker. The prose does not restate the rule, because a
  document does not block anything.
- [x] **36 new tests, 17 subtests** in `tests/test_gate_debt_precondition.py`:
  the trigger controls, the ratchet, the exemption, the exception, the
  fail-closed refusals, the generator, a test that the checker writes nothing,
  and a real `driftwatchdog gate` run.
- [x] **Negative control, measured.** Weakening the trigger to grandfather every
  change (`if baseline is not None: continue`) turns **9 of the 36 tests red**,
  including the end-to-end gate acceptance. Restored: **36 passed**.
- [x] **`tests/test_promote_project.py`'s sandbox carries every fail-closed
  governance data file, and can no longer fall silently behind a new one.**
  The one-line remedy for the 7 red `test_promote_project` failures was to add
  the three records to that fixture's copy loop, but the loop was hand-kept in
  three separate blocks and guarded by `if source.is_file()`, so a file that
  failed to copy was indistinguishable from one that was never needed. It had
  already broken twice on this exact axis: `rules-drift-baseline.json` and now
  these three. The three blocks are one declared set,
  `GOVERNANCE_ROOT_DATA_FILES`, copied unconditionally, with a missing declared
  file raising instead of skipping. `projects.json` and `decisions.json` are
  excluded in a second table, each with its evidence, and
  `SandboxCoverageTests` (3 tests) fails if a top-level `.json` reaches the
  governance root and is in neither table. **Negative control, run:** a probe
  file `gate-debt-probe.json` at the root turns the coverage test red naming
  the file and the remedy, where before it would have surfaced as 7 unrelated
  exit-code failures; the probe was removed and the suite returned to green.
  The three exit-code assertions were checked against `promote_project`'s own
  contract rather than against what the code happened to return: the module
  defines exactly two codes, `EXIT_OK = 0` and `EXIT_REJECTED = 2`, and the
  tests assert 0 for a completed promotion and 2 for each refusal.
- [x] **No sibling repository is written** except the one line in
  `driftwatchdog` this rule cannot gate without, and the one test beside it.
- [x] **No `rm`.** Fixtures use `tempfile.TemporaryDirectory()` and
  `Path.unlink(missing_ok=True)`.

### 9.1 Measured evidence

| Measurement | Value |
|---|---|
| Grandfathered pairs generated | 39 across 22 repositories |
| `gate_debt_baseline.py --check` | up to date, exit 0 |
| `workspace_check.py --self-check` | `errors=0 warnings=0`, exit 0 |
| Fleet audit | 279 errors / 20 warnings, **0 of them this rule's** |
| The checker alone on the gate fixture | 1 finding of the new code, 91 errors / 2 warnings, exit 1 |
| `driftwatchdog gate` on the fixture, **before** the declaration | `governance-quality` `PASS`, "0 of 93 governance findings matched this concern's scope", gate exit 0 |
| `driftwatchdog gate` on the fixture, **after** the declaration | `governance-quality` `FAIL`, "1 of 93", rule `QUALITY_NEW_CHANGE_WITH_GATE_DEBT`, gate `FAIL`, `blocked: true`, exit 1 |
| `driftwatchdog` suite | **909 passed, 0 failed** (908 before, +1 new test) |
| `python3 -m unittest discover -s tests` | **Ran 800 tests, OK** |
| `python3 -m pytest tests -q` | **800 passed, 175 subtests passed** |
| Suite before this change | 760 passed, 175 subtests, 1 failed |

### 9.2 The pre-existing failure, and who fixed it

The session's first baseline run failed
`SequencingGateTests::test_the_gate_is_open_and_the_fleet_corpus_is_clean` on
`lileililiwen-site`. It is a live-fleet assertion, not a stable identity: later
runs failed `test_the_thirty_disagreements_resolve_to_the_decided_table`, and
with this change stashed the suite still failed
`test_ruling_four_keeps_the_lileililiwen_id_and_path`. It was fixed by the
owner's own concurrent commits (`f192771`, `57dd605`), not here, and nothing in
this change narrows what those tests check.

### 9.3 Deliberately not done

- [x] The checker never prunes grandfathered pairs. After a fresh dry run and
  the debt driver both measured zero outstanding debt, rerun the baseline
  generator explicitly: `gate-debt-baseline.py --check` now reports 0 pairs
  across 0 debted repositories.
- [x] Keep unrelated fleet findings outside this change. The governance
  self-check passes; the full portfolio audit remains a separate owner queue.
- [x] Archive authorization is recorded in the owner's current request.
