## ADDED Requirements

### Requirement: The real shape is detected from disk

A repository's project shape MUST be determined by reading its files, and
MUST NOT be read from the `stack` label in `.project.json`, because that
label disagrees with the disk for a large minority of the fleet. The
declared `stack` MUST NOT be passed into detection at all. Detection MUST
recognise, at any depth, a `*.sln` or `*.slnx` solution, a `*.csproj`
tree, a `Cargo.toml` together with whether it declares `[workspace]`, a
`package.json` together with which of its `scripts` keys exist, Python
test configuration, and a `pubspec.yaml`. A directory walk MUST NOT enter
`.git`, `node_modules`, `target`, `bin`, `obj`, `.venv`, `venv`, `dist`,
`build`, `.next`, `site-packages`, `.dart_tool` or the language caches, so
that a vendored file cannot be mistaken for the repository's own evidence.

`.slnx` MUST be treated as a solution on the same footing as `.sln`,
because repositories that use it pin an SDK that reads it natively and a
`*.sln`-only search finds no solution in them at all.

#### Scenario: The label is wrong and the files are not

- **WHEN** a repository declares `stack: typescript` and carries a
  solution file at its root alongside a nested `package.json`
- **THEN** the solution is the detected shape and no command is derived
  from the declared label

#### Scenario: A solution in the new file format

- **WHEN** a repository carries `LoreMix.slnx` and no `*.sln`
- **THEN** `LoreMix.slnx` is detected as the solution, and a command
  derived from it names `LoreMix.slnx`

#### Scenario: A vendored file is not evidence

- **WHEN** the only `conftest.py` in a repository is under
  `.venv/…/site-packages/`
- **THEN** it is not read, and it does not make the repository a pytest
  project

### Requirement: A command is derived per concern from the detected shape

For each concern a repository's manifest already selects, a command MUST
be derived from the detected shape and the files present, and never from
a default. A `.sln` or `.slnx` solution MUST yield `dotnet build <entry>`
and `dotnet test <entry>`; a `[workspace]` `Cargo.toml` MUST yield
`cargo build --workspace` and `cargo test --workspace`; a `Cargo.toml`
without `[workspace]` MUST yield `cargo build` and `cargo test`; a
`package.json` MUST yield `npm run build` and `npm test` **only** when
the `build` and `test` script keys respectively exist; a Python
repository with pytest configuration MUST yield `pytest`. A manifest
nested below the repository root MUST be reachable, and the derived
command MUST carry the working directory that reaches it. A `.sln` entry
point MUST be the solution when one exists, and otherwise exactly one
`.csproj`; a repository with several `.csproj` files and no solution MUST
NOT have one chosen for it.

#### Scenario: The workspace flag follows the manifest

- **WHEN** a repository's `Cargo.toml` declares `[workspace]`
- **THEN** the derived `build` command is `cargo build --workspace` and
  the derived `tests` command is `cargo test --workspace`

#### Scenario: No workspace flag means no flag

- **WHEN** a repository's `Cargo.toml` declares no `[workspace]`
- **THEN** the derived `build` command is `cargo build`, not
  `cargo build --workspace`

#### Scenario: The exact script key is used

- **WHEN** a repository's `package.json` declares `scripts.test`
- **THEN** the derived `tests` command is `npm test`, and the command
  names the script key that exists on disk

#### Scenario: A nested manifest carries its directory

- **WHEN** the only `Cargo.toml` is at `rust/Cargo.toml`
- **THEN** the derived `tests` command begins `cd rust &&` and names the
  nested manifest

### Requirement: An unknown shape is refused, never guessed

When no command can be derived for a concern, the concern MUST be
recorded as `unresolved` with the reason and the evidence, and no command
MUST be written. This applies to an unrecognised shape, a `package.json`
that lacks the required script key, an undetectable test runner, a
`package.json` without a `build` script where a build was asked for, a
pure-Python repository where a `build` was asked for, and a Flutter
repository where a target platform would have to be invented. A wrong
command MUST be treated as worse than an honest gap. The tool MUST NOT
read a repository's `notes:` prose or any other free text to recover a
command.

#### Scenario: The script key does not exist

- **WHEN** a repository's `package.json` has no `test` key and the
  manifest selects `tests`
- **THEN** `tests` is `unresolved` with the reason naming the missing
  `scripts.test` key, and no `commands.tests` entry is written

#### Scenario: A pure-Python repository has no build step

- **WHEN** a Python repository's manifest selects `build`
- **THEN** `build` is `unresolved` with the reason stating that a
  pure-Python package has no build step, and no command is written

#### Scenario: A test runner cannot be determined

- **WHEN** a repository has test files but none that `pytest` or
  `unittest` can be shown to collect
- **THEN** `tests` is `unresolved` with the reason naming what was
  looked for

### Requirement: Two equally plausible entry points stay unresolved

Where a repository contains more than one entry point that can answer the
same concern, the concern MUST be `unresolved` and the reason MUST name
every candidate, unless exactly one candidate is at the repository root.
Candidates that cannot answer the concern MUST be excluded before the
root-versus-nested comparison is made. There MUST be no tiebreaker and no
preference for one toolchain over another, because a tie means the
repository genuinely has two answers and choosing one would assert a shape
it does not have.

#### Scenario: A root entry point wins over a nested one

- **WHEN** a repository has a solution at its root and a `package.json`
  with a `test` script in a subdirectory
- **THEN** the solution provides the `tests` command and the nested
  manifest is not consulted

#### Scenario: Two nested candidates are a refusal

- **WHEN** a repository has a solution in one subdirectory and a
  `package.json` with a `test` script in another, and neither is at the
  root
- **THEN** `tests` is `unresolved` and the reason names both candidates

#### Scenario: Two root candidates are a refusal

- **WHEN** a repository has both a solution and a `package.json` with a
  `test` script at its root
- **THEN** `tests` is `unresolved` and the reason names both

#### Scenario: A candidate that cannot answer is not a candidate

- **WHEN** a repository's root `package.json` has no `test` script and a
  nested Rust workspace does
- **THEN** the nested workspace provides the `tests` command, and no
  ambiguity is reported

### Requirement: A binding is only written for a concern the gate already requires

A command MUST be written only for a concern the gate would report as an
unbound required check for that repository — one the manifest listed under
`checks:`, or one its `profile:` schedules and the manifest did not disable,
exempt, or already bind. The tool MUST NOT remove a concern, MUST NOT change
an existing `checks:` value, and MUST NOT turn a disabled concern on. A
concern that is disabled under `checks: <id>: false`, exempted under
`not_applicable:`, marked `"optional"`, or absent from both `checks:` and the
profile's defaults MUST be left alone whether or not a command could be
derived for it. A command already present for a required concern MUST be left
byte-for-byte alone when it equals the derived command, and MUST be reported
as a conflict and left untouched when it differs, because an existing binding
is a decision somebody made by hand.

**Amended.** This requirement previously read "Only a concern the repository
declared is bound", and forbade the tool from touching `checks:` at all. That
forbade the correct action for a check the repository had already committed to
by naming a profile, and it guarded the wrong boundary: the boundary the gate
actually enforces is *`commands:` must not name a check that is not selected*,
not *`checks:` must never be written*. A manifest that selected a check only
through `profile:` and carried a `commands:` entry for it was unparseable, and
the tool's answer was to leave the gap open — which is what the fleet
measured. The narrower invariant is the one below, and the change to the
`checks:` block is permitted **only** under the conditions in the next
requirement.

#### Scenario: A concern nobody asked for stays unbound

- **WHEN** a repository's manifest does not select `format`, does not disable
  it, and its `profile:` does not schedule it, and its `Cargo.toml` would
  allow a `format` command to be derived
- **THEN** no `commands.format` entry is written and no `checks.format` entry
  is written

#### Scenario: A disabled concern stays disabled

- **WHEN** a repository's manifest records `build: false`
- **THEN** no `build` command is written and the `false` is not changed

#### Scenario: A hand-made binding is not overwritten

- **WHEN** a repository's `commands.tests` differs from the derived
  `tests` command
- **THEN** the concern is reported as a conflict, the existing command is
  left exactly as it is, and the file is not written

### Requirement: A profile-scheduled check is selected explicitly only when its command is really written

The gate's `.ai-gate` reader refuses a document whose `commands:` names a
check the manifest does not select
(`commands.<id> references a check that is not selected`,
`driftwatchdog/src/gate/aigate.rs:213`), so a `commands:` entry for a check
only a `profile:` selected cannot be written on its own. The tool MAY close
that gap by writing **both** a `checks: <concern>: true` entry and a
`commands: <concern>` entry in the same write, but only when every one of the
following holds. Failing any one of them, the concern MUST stay unbound and
MUST land in the debt record exactly as it does today.

1. The concern is a **required** check for this repository under the gate's
   own resolution — scheduled by the profile and not disabled, exempted, or
   already bound.
2. A command is **actually derivable** for it. Reachability is measured
   against files on disk, not assumed: the checker script exists at the path
   the command names, relative to the repository root.
3. The `commands:` entry **is written** in the same operation. A `checks:`
   entry with no matching command is a strictly worse state than the gap it
   replaced, because it makes a declared check the gate now fails on while
   still running nothing.

The `checks:` entry MUST be written with the value `true`, MUST be inserted
into the existing `checks:` block in the position the profile schedules it,
and MUST NOT be written twice when the repository already lists the concern.

**A `quality` policy is derived from disk, and the judgement in it is stated
as a default.** `placeholder-threshold` and `product-code-boundary` are
answered by `product_code_quality.py`, which is fail-closed: without a
`quality` block in the repository's own `.project.json` it exits `2`
(configuration error) and never scans. Two of the fields in that block are
derivable from the files on disk and are derived, the way a build command is
derived and with the same on-disk evidence attached; the third is a judgement,
and is written as a default a project may relax deliberately rather than as a
neutral one. The two are not treated alike, because they are not alike: a
product root is a fact about where the code is, and a threshold is a decision
about how much unfinished work to tolerate.

The block is written and the command bound in **one** operation, and the
concern counts as bound only once the block is on disk. Until then the
concern is debt, because a `commands:` entry whose checker still exits `2`
for want of the policy it reads is a bound check that runs nothing — the same
defect as a `checks:` entry with no command behind it, and refused for the
same reason.

#### Scenario: A profile-scheduled check with a reachable checker is bound in both blocks

- **WHEN** a repository's manifest says `profile: product`, does not list
  `governance-quality` under `checks:`, and the governance checker script
  exists on disk at the path the derived command names
- **THEN** both a `checks: governance-quality: true` entry and a
  `commands: governance-quality` entry are written, and the concern is no
  longer in the debt record

#### Scenario: A profile-scheduled check with a derivable quality policy is bound in both blocks

- **WHEN** a repository's manifest says `profile: product`, does not list
  `placeholder-threshold` under `checks:`, no `quality` block exists in its
  `.project.json`, and a product root and language set are both derivable from
  the files on disk
- **THEN** a `quality` block naming the derived roots and languages and a
  `placeholder_threshold` of `0` is written to `.project.json`, both a
  `checks: placeholder-threshold: true` entry and a
  `commands: placeholder-threshold` entry are written in the same operation,
  and the concern leaves the debt record

#### Scenario: A concern the repository already declares is not duplicated

- **WHEN** a repository's manifest already lists `governance-quality: true`
  under `checks:`
- **THEN** no second `governance-quality` entry is added to `checks:`, and
  only the `commands:` entry is written

#### Scenario: A checks entry is never written without its command

- **WHEN** the tool writes a `checks: <concern>: true` entry
- **THEN** a `commands: <concern>` entry for the same concern is written in
  the same operation, so no manifest the tool produces can carry a selection
  with no command for it

### Requirement: The debt record is complete against the gate's own unbound set

The set of (repository, concern) pairs in the debt record MUST equal the set
the gate reports as unbound required checks, with the gate's own dry-run
output as the authority. A repository MUST appear in the record whenever the
gate reports an unbound required check for it, including when the only
unbound checks are ones a `profile:` scheduled rather than the manifest
listing, and including when the repository's manifest has uncommitted work.
A check the manifest disabled under `checks:`, a check exempted under
`not_applicable:` with a reason, and a check the gate executes in-process
MUST NOT be recorded, because the gate reports none of them as unbound.

A record that names a subset of the gaps MUST be treated as a defect rather
than as a shorter list: a reader — human or agent — cannot tell an
under-reporting record from a complete one by reading it, and a gate check
nobody can see is a gate check nobody closes.

#### Scenario: A profile-scheduled check is in the record

- **WHEN** a repository binds every check its manifest declares, and its
  `profile: product` schedules three more that bind no command
- **THEN** all three appear in that repository's entry, each with a reason

#### Scenario: A repository whose only debt is profile-scheduled is still listed

- **WHEN** a repository has no unresolved check in its own `checks:` and
  three unresolved checks from its profile
- **THEN** it appears in `repositories` with those three, rather than being
  absent for having no gaps of its own

#### Scenario: Uncommitted work does not hide debt

- **WHEN** a repository's manifest has uncommitted changes
- **THEN** the manifest is not written, and the checks the gate would report
  as unbound for it are still classified and recorded

#### Scenario: An excused check is not debt

- **WHEN** a repository exempts a check under `not_applicable:` with a reason
- **THEN** that check does not appear in the record, because the gate does
  not fail on it

#### Scenario: The record and the gate agree

- **WHEN** the tool runs over the fleet and the gate is available
- **THEN** no (repository, concern) pair the gate reports as unbound is
  missing from the record, and no pair in the record is absent from the
  gate's report

### Requirement: A run is dry by default and refuses a dirty manifest

The tool MUST NOT write any file without `--apply`, and a run without it
MUST report what it would do and change nothing on disk. A repository
whose `.ai-gate/gate.yaml` has uncommitted work MUST be skipped rather
than written into, including when that manifest is untracked, and an
unreadable or unparseable manifest MUST be refused by name rather than
rewritten. A run MUST continue past a refused repository to the next one.

#### Scenario: Dry run changes nothing

- **WHEN** the tool runs without `--apply`
- **THEN** every repository's manifest is byte-identical afterwards, and
  the report names what would be bound

#### Scenario: A dirty manifest is skipped

- **WHEN** a repository's `gate.yaml` is modified but uncommitted
- **THEN** that repository is reported as dirty and its manifest is not
  written

### Requirement: Every unresolved check leaves a durable debt record

For every repository that still cannot be fully wired, the tool MUST
leave a machine-readable record naming every unresolved concern and the
reason for each, so that the gap survives the run and cannot be inherited
silently. The record MUST be a committed JSON data file at the governance
root, and MUST hold per-repository entries with the concern id and its
reason, a grouping of the same reasons with the repositories they affect,
and totals. The record MUST NOT carry a timestamp, and every list MUST be
sorted, so that re-running the tool on an unchanged fleet produces a
byte-identical file and a change in the file is a real change in the
debt. The record MUST be written only under `--apply`, and a person
opening it MUST be able to see the grouped reasons before the repository
names.

#### Scenario: The debt names every gap

- **WHEN** a repository has three concerns the tool cannot resolve
- **THEN** the record contains one entry for that repository with all
  three concern ids and their reasons

#### Scenario: The record is stable

- **WHEN** the tool is run twice with `--apply` on an unchanged fleet
- **THEN** the second run's record is byte-identical to the first

#### Scenario: A human sees the cause first

- **WHEN** the record is read
- **THEN** the grouped reasons and their check counts appear before the
  per-repository entries

### Requirement: An unresolved check names the specific missing configuration

A reason recorded for an unresolved concern MUST name the file and the key
whose absence causes the refusal, not merely the class of the problem. A
reason that says a check "needs configuration" tells the next reader nothing
they can act on; a reason that names `quality` in `.project.json` tells them
the file to open and the block to add.

Where the missing input is a declaration in a specific file, the reason MUST
name that file and that block or key. This applies in particular to
`placeholder-threshold` and `product-code-boundary`, whose checker is
fail-closed on a `quality` block in the repository's `.project.json` and exits
`2` — a configuration error — without one.

#### Scenario: The reason names the file and the block

- **WHEN** a repository has no `quality` block in its `.project.json` and its
  profile schedules `placeholder-threshold`
- **THEN** the recorded reason names `.project.json`, names the `quality`
  block, and says the checker exits 2 without it

#### Scenario: A refusal that is not about configuration names what was looked for

- **WHEN** no product root can be derived from a repository's files
- **THEN** the reason names the directories and manifests that were examined
  and says that no source root was found among them, rather than naming a
  missing configuration the reader could go and add

### Requirement: The quality block's derivable fields come from disk, and its judgement is a stated strict default

A `quality` block the tool writes MUST carry a `product_roots` list and a
`languages` list that are **derived from the files on disk**, by the same
detection the build commands use and never from the `stack` label, and each
derived root MUST be accompanied by the manifest it was derived from as
evidence. A root MUST be a directory that exists on disk, MUST be relative to
the repository root, and MUST NOT escape it. A test project, and any
directory under a `tests` or `test` path segment, MUST NOT become a product
root: the checker excludes those segments anyway, so naming one would buy a
scanned-nothing result that reads as a pass.

Where **no** product root can be derived, the tool MUST refuse by name rather
than write a block, and the reason MUST name what was examined. It MUST NOT
fall back to the repository root, to `.`, or to any directory it did not
verify contains source.

`placeholder_threshold` is a judgement rather than a derivation, and the value
written MUST be `0`. It MUST be documented as the **deliberately strict**
default — the strictest value the checker accepts — chosen because both
projects that declare the block by hand chose it, and because a check that
runs and complains is worth more than a check that does not run. It MUST NOT
be presented as a neutral default, and a project MUST be able to relax it
deliberately in its own `.project.json`, where this tool will then leave it
alone.

The tool MUST NOT invent an `exclude` list. `exclude` is a judgement about
which files are not product, and a guessed exclusion hides a real finding
rather than reporting one. It MUST be omitted unless a derivation is
mechanical, and no mechanical derivation for it exists. The same applies to
`placeholder_markers`: the checker resolves its own markers from the
workspace vocabulary, and a hand-written copy of that list in every
`.project.json` would be a second source of truth that drifts.

#### Scenario: The roots name the manifests they came from

- **WHEN** a repository's disk shows one `Cargo.toml` at `rust/Cargo.toml` with
  a `rust/src` directory
- **THEN** the written `product_roots` names `rust/src`, the `languages` list
  names `rust`, and the evidence names `rust/Cargo.toml`

#### Scenario: No derivable root is a refusal, not a fallback

- **WHEN** a repository has a `package.json` and no directory that source
  files can be shown to live in
- **THEN** no `quality` block is written, no `checks:` or `commands:` entry is
  written, and the reason names the manifests that were examined

#### Scenario: A test project is never a product root

- **WHEN** a repository's only `.csproj` files are under `tests/`
- **THEN** no product root is derived from them and the concern stays
  unresolved, rather than a block being written whose only root the checker
  would exclude in full

#### Scenario: The threshold is zero and says it is strict

- **WHEN** a `quality` block is derived
- **THEN** it carries `placeholder_threshold` of `0`, and the reason and the
  documentation describe it as the deliberately strict default a project may
  relax, not as a neutral one

#### Scenario: No exclude list is invented

- **WHEN** a `quality` block is derived
- **THEN** it carries no `exclude` key and no `placeholder_markers` key, so
  the exclusions and the marker vocabulary remain the checker's own

#### Scenario: A hand-written block is left exactly as it is

- **WHEN** a repository's `.project.json` already carries a `quality` block
- **THEN** the block is not read for its values, not rewritten, not merged
  into, and not reordered; the tool binds the command and changes nothing
  else in the file

### Requirement: The run reports per repository and refuses to exit clean on debt

The tool MUST report, for every repository, which concerns it bound, which
it left alone, and which are unresolved with the reason, and MUST roll
those up into a per-repository status of fully resolved, partially
resolved, or fully unresolved. A summary MUST make an unresolved
repository obvious. The tool MUST exit non-zero when any unresolved
concern or conflict remains, MUST NOT offer a flag that suppresses that
exit code, and MUST return a distinct code for an unusable workspace root.
A tool that reports a gap and then exits successfully discards the
finding, which is the defect ruling D18 exists to prevent.

#### Scenario: Debt cannot be ignored

- **WHEN** a run completes with at least one unresolved concern
- **THEN** the tool exits with the debt exit code and prints the number
  of repositories carrying debt

#### Scenario: A clean fleet exits zero

- **WHEN** a run completes with every declared check bound
- **THEN** the tool exits 0

#### Scenario: The roll-up is what a reader sees first

- **WHEN** a run covers a mix of fully resolved, partially resolved and
  fully unresolved repositories
- **THEN** the summary reports all three counts

### Requirement: A new change waits on the debt already recorded for its repository

A repository that carries unresolved gate debt MUST NOT open a new OpenSpec
change, and `scripts/workspace_check.py` MUST enforce that rather than only
advising it in `.ai-rules/completion.md`. The rule MUST report the code
`QUALITY_NEW_CHANGE_WITH_GATE_DEBT` at level `ERROR` when BOTH of the
following hold for one repository:

- it has at least one required check the gate reports as `UNBOUND`; and
- it has at least one unarchived OpenSpec change which is neither the change
  that closes that debt nor a `(project, change)` pair already recorded in
  `gate-debt-baseline.json`.

The baseline is a one-time grandfathering of the work that predates the rule,
in the shape of `rules-drift-baseline.json` and written from disk by
`scripts/gate_debt_baseline.py`. It is what makes the rule a ratchet instead of
a deadlock: without it, failing the 22 repositories that already carry open
changes fails the legacy fleet for the state the rule exists to stop growing,
and no action available to those repositories can clear it, because an open
change can only be closed by finishing it. A baseline entry is not an
exemption either — a repository that discharges its debt leaves
`gate-debt.json` and is never consulted — so a stale entry is dead weight
rather than a live hole, and the list only shrinks. The baseline is a
committed list a reviewer reads, not a switch: there is no environment variable
and no flag.

Debt MUST be read from `gate-debt.json` and MUST NOT be re-derived by a second
parser, because `GateOracleTest` already pins that record set-equal to the
gate's own unbound set. A repository absent from the record has no debt and
MUST NOT be reported. An archived change MUST NOT count as a new change. The
trigger set MUST be the same set of directories `validate_change_names`
validates, so the rule cannot count a directory the naming rule ignores.

The rule MUST NOT fire when the only unarchived change is the debt-closing
change itself, because a repository that may not start the work which fixes
its debt can never fix it, and the rule would be a dead end.

The finding MUST name the repository, the unbound concerns, the unarchived
change that triggered it, and the change that records the fix. A finding that
does not say what to do next is not actionable, and an unactionable blocking
finding is ignored within a week.

The code MUST be listed in `GOVERNANCE_QUALITY_DECLARED_CODES` in
`driftwatchdog`. A code outside that list is filtered out of the concern's
verdict, so the concern would report `PASS` while this checker exits 1 — the
rule would be a report line no gate reads.

#### Scenario: Debt plus a new change is an error

- **WHEN** a repository in `gate-debt.json` carries an unarchived change that
  is neither `close-unbound-gate-checks` nor a pair in `gate-debt-baseline.json`
- **THEN** the checker reports `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` at `ERROR`
  for that repository, naming the unbound concerns and the new change

#### Scenario: Work that predates the rule is not new work

- **WHEN** a repository in `gate-debt.json` carries only unarchived changes
  whose `(project, change)` pairs are all in `gate-debt-baseline.json`
- **THEN** no finding is reported, so the legacy fleet is not failed for the
  state the rule was written to stop growing

#### Scenario: Only the debt-closing change is open

- **WHEN** a repository in `gate-debt.json` has `close-unbound-gate-checks` as
  its only unarchived change
- **THEN** no finding is reported, and the debt itself is still named by the
  generator's own record rather than by this rule

#### Scenario: A repository with no debt is never reported

- **WHEN** a repository is absent from `gate-debt.json` and has several
  unarchived changes
- **THEN** no finding is reported for it

#### Scenario: The debt record cannot be read

- **WHEN** `gate-debt.json` is missing or malformed
- **THEN** the run reports one `ERROR` naming the file and the reason, and no
  repository is reported as debt-free on the strength of a record that was
  never read

### Requirement: The precondition has a declared exit, recorded as evidence

A fail-closed rule with no way to record a deliberate decision trains people to
ignore it, so the exit MUST be declared. It MUST be a committed record,
`gate-debt-exceptions.json` in the governance root, keyed by `(project,
change)` and carrying a non-empty `reason`. It MUST NOT be an environment
variable, a skip flag, or an in-memory override: a check that can be switched
off is not a check.

Both records MUST be read fail-closed, in the same shape and with the same
refusals as `rules-drift-baseline.json`: a missing file, an unreadable file, a
root that is not an object, a missing or non-list `exceptions` key, an entry
that is not an object, an entry missing a key, a non-string or empty value, or
an unknown key are each an `ERROR` naming the file and the reason. An unknown
key is refused rather than ignored, so a typo in a hand edit fails loudly
instead of silently dropping a recorded decision. An empty `exceptions` array
is valid and is the normal state.

A ratchet that could not be read MUST grandfather nothing: the already-open
work it failed to list is reported again, because a baseline that was never
read is not a baseline. The run still fails, naming the file, as every other
loader failure in this checker does.

A `(project, change)` pair present in `gate-debt-exceptions.json` MUST satisfy
the rule, and MUST still be reported, at level `WARNING`, with the recorded `reason` quoted
and the record named. The decision stays review-visible in the run's own output
instead of becoming an invisible pass. Only `ERROR` affects the exit code, so
a recorded exception never blocks and never disappears.

#### Scenario: A recorded exception satisfies the precondition and stays visible

- **WHEN** a repository carries debt, has an unarchived change, and that
  `(project, change)` pair is in `gate-debt-exceptions.json`
- **THEN** the finding is reported at `WARNING` quoting the recorded reason,
  and the run's error count is unchanged

#### Scenario: An exception does not cover a change nobody recorded

- **WHEN** a repository carries debt and records an exception for one
  unarchived change while another unarchived change is unrecorded
- **THEN** the unrecorded change is reported at `ERROR`

#### Scenario: A typo in the record fails loudly

- **WHEN** an entry in `gate-debt-exceptions.json` carries a key other than
  `project`, `change` or `reason`
- **THEN** the run reports an `ERROR` naming the file, the entry and the
  unknown key, and honours no exception at all
