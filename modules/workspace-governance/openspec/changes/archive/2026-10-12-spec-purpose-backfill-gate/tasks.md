# Tasks: spec-purpose-backfill-gate

## 0. Caveats and confirmations

- [x] **The measurement is re-verified, and the earlier figure was stale.** 79
  repositories carry an `openspec/specs` tree holding **1,958** spec files, of
  which **1,234** hold the marker, across 71 repositories. 1,229 carry the
  current one-line form and 5 carry the older short form the archiver used
  before it learned to append `Update Purpose after archive.`. 7 of the 1,234
  carry a blank line between the heading and the marker. The checkpoint's
  1,236/1,957/78 figures are superseded: two markers were resolved in the live
  fleet in the interim and the earlier count did not include the repository
  whose specs tree appeared with it. Re-measured two independent ways, agreeing.
- [x] **The marker holds the bare change name; the archive directory is
  date-prefixed.** Verified across the 1,161 markers whose archive directory
  resolves: the marker text is `created by archiving change <name>` with no
  date, while the directory is `YYYY-MM-DD-<name>`. The obvious implementation —
  joining the two — resolves **zero** files. Resolution strips the date prefix
  and indexes the remainder.
- [x] **The `## Why` count is threshold-dependent and is now recorded.** 1,161
  markers name an existing archive directory; 1,159 have a `## Why` at all and
  1,089 clear the 120-character floor this change adopts. The prior art's
  absence of a `## Why` source is why the earlier "1,049 recoverable" figure
  understates it: that figure was measured against a tool that never looked at
  the archive.
- [x] **`SPEC_PURPOSE_PLACEHOLDER` is ERROR and the fence stays closed.** 1,234
  violations exist, so the rule is held off the fleet by
  `SPEC_PURPOSE_AUDIT_ENABLED = False` — a committed, repo-relative constant,
  never an environment variable. The severity is **not** softened to WARNING to
  achieve the hold, and no second fencing mechanism is added.
- [x] **D18 is NOT satisfied by this change, and that is recorded as a blocker.**
  The `governance-quality` concern's effective code scope is the closed
  allow-list `GOVERNANCE_QUALITY_DECLARED_CODES` (4 codes, not including this
  one); the adapter filters out-of-scope findings at `mod.rs:453`; and because
  the checker's exit 1 is an *expected* exit (`mod.rs:84`), the concern would
  report **PASS while the checker fails**. This change does not edit
  `driftwatchdog` and does not claim the rule is gated. See `design.md` §6.
- [x] **The derived sentence is preserved byte-for-byte.** The
  requirement-derived fallback stays exactly as `fill_spec_purposes.py` produces
  it, including the verbatim `kebab-case` capability name, so the fleet write is
  reproducible against what that script already committed in `openlearning`.
  Changing the wording would make the shared tool's output differ from the
  result the prior art is the record of.
- [x] **An ambiguous change name refuses the plan, row-level.** Two specs resolve
  to two archive directories each; the tool names both and writes nothing.
  Choosing between them is a guess about authorship whose output is a published
  sentence. The refusal stays row-level so the other 1,232 rows remain
  reviewable, while `--apply` stays all-or-nothing.
- [x] **No sibling repository and no archiver is edited.** `@fission-ai/openspec`
  is third-party; the durable fix is the tool plus the rule. The 1,234-file write
  is a separately authorized cross-repo step, as the axis migration was.
- [x] **Stage only this change's files.** No `git add -A`; every path is staged
  explicitly. `rm` is never invoked — cleanup uses
  `tempfile.TemporaryDirectory()` and `shutil.rmtree(..., ignore_errors=True)`.

## 1. BFS — Blast radius and prior art

- [x] **Read the prior art and judge it.** `openlearning/scripts/fill_spec_purposes.py`
  implements the detection pattern, the `### Requirement:` extraction, the
  capability-name resolution and both derived sentences. Its reach, its safety
  contract and its ambiguity handling are not, and `design.md` §4 states each
  change and why.
- [x] **Reuse the shipped tool shape rather than inventing one.**
  `scripts/axis_migration.py` is the contract: dry run is the only default,
  `--apply` is the sole write path, `--check` exits 3 while outstanding, and the
  apply path prints the flag reminder. Copied shape, not copied code.
- [x] **Enumerate the fleet surface before writing.** 79 repositories with a
  specs tree, 1,958 spec files, 1,234 markers across 71 repositories, 1,161
  resolvable archives, 1,089 usable `## Why` sections, 2 ambiguous names.
- [x] **Confirm the checker rule's insertion point.** `validate_project` calls
  `validate_change_names` for the openspec surface; the new
  `validate_spec_purposes` sits beside it and is deliberately **not** gated on
  `requires_openspec`, because a descriptive rule must not become a
  requirements driver.
- [x] **Confirm the 501-test baseline and the 205/18 audit baseline** before
  changing anything, so an unrelated movement is attributable.
- [x] **Verify the D18 claim instead of asserting it.** Read
  `driftwatchdog/src/gate/governance/mod.rs` and `manifest.rs` read-only. The
  claim is false; `design.md` §6 carries the citations and §6.1 the structural
  weakness and the candidate guard.

## 2. DFS — The tool, the rule, the fence

- [x] **`scripts/spec_purpose_backfill.py` exists and follows the shipped shape.**
  `--root`, `--json`, `--check`, `--write-plan`, `--apply`; a `BackfillError`
  refused to `stderr` with exit 1; exit 3 from `--check` while anything is
  outstanding.
- [x] **Dry run writes nothing, provably.** The fleet run reports 1,234 rows and
  the tool writes no file; the dry run prints
  `Re-run with --apply under explicit authorization.`
- [x] **The `## Why` source is preferred and recorded.** Each row carries
  `source: archived-change-why` with the evidence path, or `first-requirement`,
  or `capability-name`.
- [x] **The archive resolution handles the date prefix and refuses ambiguity.**
  Ambiguous names are refused at row level and named with both directories.
- [x] **`MIN_WHY_CHARS = 120` is a named constant**, recorded in the plan and
  asserted at its exact boundary by a test, because the boundary is a judgment
  call that must be visible.
- [x] **The `--apply` path prints the flag reminder**, naming
  `SPEC_PURPOSE_AUDIT_ENABLED = True` in `scripts/workspace_check.py` in the
  same commit, and names the `driftwatchdog` dependency that must land first.
- [x] **`SPEC_PURPOSE_PLACEHOLDER` is ERROR** in `scripts/workspace_check.py`,
  naming the spec file relative to its repository and the change the marker
  records, and firing for both marker generations.
- [x] **The fence is one committed constant and nothing else.**
  `SPEC_PURPOSE_AUDIT_ENABLED = False` plus a `spec_purpose_audit_enabled()`
  resolver matching `taxonomy_audit_enabled()` exactly. No environment variable,
  no skip flag, no exemption registry.
- [x] **The rule is skipped for a repository with no `openspec/specs` tree**, so
  the non-OpenSpec repositories are untouched.
- [x] **`validate_project` gains one defaulted keyword argument**, so every
  existing caller is unaffected and the checker diff stays minimal.
- [x] **`--check` answers all three states.** Not applied exits 3, applied exits
  0, re-applied is idempotent at 0. The checkpoint's version applied the drift
  refusal to `--check`, so a fully applied fleet read as 1,234 rows of drift and
  the tool could never report "applied"; `build_plan` now takes
  `verify_committed` and `--check` passes `False`. `design.md` §5.1.
- [x] **The marker must be the whole `## Purpose` section, not merely a line.**
  The checkpoint's `[^\n]*` matched a real sentence appended after the marker,
  contradicting its own comment and the spec delta's scenario. The name is now
  bound to `[\w.-]+` and the body is matched as a section; measured, this still
  matches exactly the same 1,234 fleet files and no others. `design.md` §2.

## 3. BFS — Re-audit every consumer

- [x] **The fleet audit is unmoved with the fence closed:**
  `errors=205`, `warnings=18` before and after. The new rule emits nothing.
- [x] **`--self-check` stays 0 errors / 0 warnings.**
- [x] **The full suite is 501 before and 553 after** (52 new tests), with zero
  failures and no existing assertion weakened, relaxed, or deleted.
- [x] **The fence test proves both states on the same unmigrated corpus:** with
  the constant `False` no finding is reported, and flipping it to `True` reports
  ERROR for every placeholder and turns the gate red.
- [x] **The same property is measured on the real fleet, not only a fixture:**
  the audit goes from 205 errors to **1,439**, which is 205 plus exactly the
  1,234 markers. The constant is committed back at `False`.
- [x] **A test proves the environment variable cannot open the fence**, mirroring
  `test_the_gate_is_a_committed_constant_not_an_environment_variable`.
- [x] **A test proves the fallback derivation matches the prior art's exact
  string**, which is the reproducibility property the fleet write rests on.
- [x] **A test proves the checker and the tool share one marker pattern**, which
  is the condition for a resolved spec not still reporting ERROR.
- [x] **The drift refusal is covered by a fixture test.** It is inert against
  the live fleet because no plan is committed, and an inert mechanism tested only
  against the fleet would be untested.
- [x] **`docs/vocabulary.md` gains the marker, the finding code, the fence and the
  sequencing steps**, beside the taxonomy section it parallels; `README.md` gains
  one pointer line and no restatement.
- [x] **`openspec validate spec-purpose-backfill-gate --strict` passes** and
  `git diff --check` is clean.

## 4. Unchecked, and why

These five were open when this change was first committed. Two have since been
completed by separately authorized steps; the record is updated rather than
rewritten, so the original reasoning stays visible.

- [x] **D18 is now met.** The `governance-quality` concern's declared code set
  did not include `SPEC_PURPOSE_PLACEHOLDER`, the adapter filtered it out
  (`mod.rs:453`), and the checker's exit 1 was an expected exit (`mod.rs:84`),
  so the concern reported `PASS` while the checker failed. **Landed in
  `driftwatchdog`:** `SPEC_PURPOSE_PLACEHOLDER` was added to
  `GOVERNANCE_QUALITY_DECLARED_CODES` (`6390ab9`), and the before/after was
  measured on a real binding — `PASS`/exit 0 became `FAIL`/exit 1 with the
  finding reported. `SPEC_PURPOSE_AUDIT_ENABLED` is therefore `True`.
- [x] **Carry the declared-code allow-list weakness as a separate
  Driftwatchdog follow-up.** The placeholder code itself is now admitted by
  the gate and was verified in a real binding. A complete inventory guard
  requires Driftwatchdog's Rust adapter and a new envelope contract; this
  repository does not own that runtime change. The residual risk and proposed
  `emittable_codes` contract remain explicit in `design.md` §6.1 and HANDOFF.
- [x] **The original 1,234-file fleet write is applied.** It was authorized
  and run with `python3 scripts/spec_purpose_backfill.py --apply
  --resolve-ambiguous-as-derived`, then committed across 70 repositories;
  `rollback-manifest.json` records before/after SHAs. The original post-write
  measurement was 0 markers. A fresh check on 2026-10-12 now finds 134 newer
  markers across sibling repositories. Those later archives are separate
  cross-repository work and remain visible for owner-authorized cleanup; this
  record does not claim the current fleet is marker-free.
- [x] **The two ambiguous rows are resolved, by declining both archives.**
  `dotnet-platform-libs` `platform-testing-toolkit` and `rust-ecommerce`
  `audit-log-immutability` each match two archive directories holding two
  genuinely different changes. The tool still refuses by default. The owner
  opt-in `--resolve-ambiguous-as-derived` derives the Purpose from the spec's
  own first requirement instead, which asserts nothing about authorship, and
  records the declined candidates in `evidence` so the reader still sees them.
  Covered by `AmbiguousAsDerivedTests`.
- [x] Archive with the owner's authorization. D18 wiring is measured as met;
  the separate allow-list inventory weakness remains documented above.

## 5. Two defects found while applying, both fixed at the cause

1. **`re.sub` read archived prose as a replacement template.** The write used a
   string replacement, so a backslash in a `## Why` (a regex, a Windows path) was
   parsed as an escape and raised `re.error: bad escape \s` part-way through the
   fleet, leaving 320 of 1,234 files written. The replacement is now a function,
   so the text is inserted literally, and the tool is resumable: re-running
   planned only the rows still holding a marker. Covered by
   `BackslashInArchivedProseTests`, which reproduces the original exception
   against the previous behaviour.
2. **The 2 ambiguous rows blocked the whole write.** Resolved as described above
   rather than by deleting rows from the plan.
