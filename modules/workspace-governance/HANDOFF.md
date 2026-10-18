# Workspace Governance Handoff

## Current state

current_spec: none

## Completed: interface declaration and boundary findings

`interface-declaration-and-boundary-findings` is archived at
`openspec/changes/archive/2026-10-17-interface-declaration-and-boundary-findings/`.
The portfolio had a boundary language for capabilities and none for
interfaces, so every cross-project call was invisible to governance. The
optional `interfaces[]` block now declares each boundary with its mechanism,
its contract and, where the mechanism requires one, a `sha256` digest, and the
audit charges an undeclared or unpinned edge.

**What landed.** `schemas/project.schema.json` gains `interfaces` plus the
`interface*` `$defs`; `scripts/workspace_check.py` gains the whole-declaration
validation pass (a malformed declaration yields no interface assertion, never a
partial read), cross-project resolution of a `consumes` edge against the
provider's own `provides` entry, the seven interface findings, and
`CAPABILITY_PROVIDER_UNVERIFIED`, which names the provider holding each of the
nine `exempt` capabilities unverified without moving a posture or charging a
consumer; `scripts/bind_gate_commands.py` and `scripts/bind_ui_gate.py` write
the matching `mechanism: sibling` declaration, digest included, in the same
operation as the command they write, and leave a concern unbound (recorded as
debt) rather than declare an edge they cannot describe; `decisions.json` `D7`
records 2027-01-15 as the date the two ramped findings become blocking, and the
audit reads that date from the register, so no environment variable or
project-controlled switch can start the ramp early. The declaration, the closed
mechanism set and the two lawful alternatives to a sibling path are documented
in `docs/capabilities.md`, with `D7` restated in
`docs/toolchain-decisions.md`.

**Verification.** Full unit suite — 1011 passed, 2 skipped (948 before the
change, so 63 new tests are counted); workspace self-check — 0 errors,
0 warnings; auto-gate — PASS; strict OpenSpec validation — 26 canonical specs,
0 failed; change-name validation and `git diff --check` — clean. Fleet,
measured with `scripts/workspace_check.py --root .. --json` before and after:
317 errors / 24 warnings → 317 / 33, the entire delta being the nine
design-mandated `CAPABILITY_PROVIDER_UNVERIFIED` warnings, with **zero**
`INTERFACE_*` findings on the 82 existing declarations. Generator round trip,
on a temporary scaffold only: exactly one declared `sibling` edge beside the
bound command, its digest `sha256(entry)`, recomputed and accepted by the
audit; the pinned form of a real `aegify` binding is silent, the unpinned form
reports WARNING with both lawful alternatives and never ERROR.
`bind_gate_commands.py --apply` was never run against `/home/paul/code`.

**Archive.** Archived without `--skip-specs`: `capability-adoption-gate` gained
three requirements (6 → 9) and `platform-capability-manifest` two (3 → 5), both
retaining their existing real Purposes — the archiver created no placeholder.
Its trailing blank line was removed from the adoption-gate spec so
`git diff --check` stayed clean. The package's own baseline task records the
sibling bindings measured today: **65** gate manifests bind
`workspace_check.py` and **44** bind `product_code_quality.py` (the proposal's
67 predates this run). Forge's two vendored copies
(`scripts/governance/product_code_quality.py`, `scripts/governance/vocabulary.py`)
are byte-identical to this repository's files; declaring them as
`mechanism: consume` with their digests is Forge's own follow-up change.

**Deferred scope.** Five tasks are unchecked, all of them in the package's
"Follow-up changes (other repositories, not this one)" section: Forge's
consumption change, the `platform-contracts` interface envelope,
`dotnet-platform-libs`' verified capability entries that unfreeze the nine
exemptions, `driftwatchdog`'s released checker manifest, and the per-consumer
conversion of sibling edges to digest-pinned copies or published artifacts.
None is work inside this repository's contract.

**Commits.** `a55fbb7` contains the schema, audit, both generators, the `D7`
decision, the documentation, the tests, and the archived change. This
HANDOFF-only commit records the verification and clears the active pointer;
there are no active OpenSpec changes.

## Completed: OpenSpec seriality guard

`enforce-openspec-seriality` is archived at
`openspec/changes/archive/2026-10-12-enforce-openspec-seriality/`. The
`workspace-governance` audit now reports `OPENSPEC_CHANGE_SERIALITY` when more
than one change is active or when the single `current_spec` pointer is missing,
duplicated, or stale. A live repository-state assertion in the required unit
test Gate blocks those states.

**Verification.** Focused seriality suite — 10 tests passed; full unit suite —
948 passed, 2 skipped; workspace self-check — 0 errors, 0 warnings; auto-gate —
PASS; Driftwatchdog Gate — PASS; strict OpenSpec validation — 26 passed, 0
failed; change-name validation and `git diff --check` — clean; `openspec list` —
no active changes. Driftwatchdog's declared-code allowlist does not include the
new finding code, so the local required test Gate enforces live seriality; the
checker still emits the finding in its audit output. No sibling repository was
changed.

**Commits.** `5e6b4ce` contains the implementation, canonical requirement, and
archived change. This HANDOFF-only commit clears the active pointer.

## Completed: capability adoption gate

`capability-adoption-gate` is archived at
`openspec/changes/archive/2026-10-12-capability-adoption-gate/`, with its
contract at `openspec/specs/capability-adoption-gate/spec.md`. Its generated
Purpose was replaced with a contract summary before commit. Task 4.4 is
complete: after the adoption-grant package landed,
`python3 scripts/check_skill_refs.py` passed for all nine references.

**Deferred scope.** Six unchecked tasks remain explicit follow-ups: capability
vocabulary unification, behavioral proof of adoption, legacy `kind` cleanup,
registration of four discovered repositories, Driftwatchdog gate-check
binding, and adoption-grant remediation permission (now separately specified
and archived). None is an incomplete task inside this capability contract.
OpenSpec emitted a non-blocking warning that the proposal Why section exceeds
1000 characters.

**Commits.** `7a34715` contains the canonical spec and archive. This
HANDOFF-only commit records completion and clears the active pointer; there
are no active OpenSpec changes.

**Final verification.** The first workspace self-check caught the generated
Purpose placeholder in the archived `readme-claim-verification` spec. Commit
`33fd932` replaced it with the actual contract purpose. The workspace
self-check then passed with 0 errors and 0 warnings; auto-gate passed; strict
validation passed for all 26 canonical specs; the full unit suite passed 938
tests with 2 skipped; the skill reference check passed; and `openspec list`
reported no active changes. The pre-existing dirty files and untracked
`HANDOVER.md`/`driftwatch.toml` were preserved and not staged.

## Completed: cross-repo adoption grant

`cross-repo-adoption-grant` is archived at
`openspec/changes/archive/2026-10-12-cross-repo-adoption-grant/`, with its
contract at `openspec/specs/cross-repo-adoption-grant/spec.md`. The generated
canonical Purpose was replaced with a summary of the bounded, single-use
permission before commit.

**Deferred scope.** The six unchecked tasks remain explicit follow-up work:
grant issuance, the first consumer adoption, two consumer fixture
consolidations, a Driftwatchdog gate-check adoption, capability-vocabulary
unification, and a checked assessment-name rule. These are separate owner or
sibling-project changes, outside this governance contract. OpenSpec warned
that the proposal Why section exceeds 1000 characters; archive validation and
the spec update completed.

**Commits.** `482ccbb` contains the canonical spec and archive. This
HANDOFF-only commit records the scope boundary and advances the pointer to
`capability-adoption-gate`.

## Completed: README claim verification

`readme-claim-verification` is archived at
`openspec/changes/archive/2026-10-12-readme-claim-verification/`. Its new
capability requirements are recorded in
`openspec/specs/readme-claim-verification/spec.md`; the JSON report extension
updates the existing `product-code-quality-boundary` capability.

**Archive notes.** The outstanding documentation task was already satisfied
in the live README and product quality guide; it is now checked off. The
initial archive attempt exposed a delta target mismatch, which was corrected
by placing the modified JSON-report requirement under its owning capability.
Strict change validation passed and the archive completed. OpenSpec emitted a
non-blocking proposal warning because its Why section exceeds 1000 characters.

**Commits.** `0aa1395` contains the spec updates and archive. This HANDOFF-only
commit records the outcome and advances the pointer to
`cross-repo-adoption-grant`.

## Completed: derived kind canonical form

`derived-kind-canonical-form` is archived at
`openspec/changes/archive/2026-10-12-derived-kind-canonical-form/`. The
canonical `portfolio-vocabulary` requirement now recognizes a kind derived
from present canonical axes while preserving the legacy vocabulary and its
existing scenarios.

**Archive notes.** OpenSpec validation passed. The archive command reported
one intentionally unchecked design decision (keeping `kinds` at 12) and a
non-blocking proposal heading warning (`What Changes`); the implementation,
tests, and change scope were already complete. The archive preserved both
pre-existing canonical scenarios that the initial delta omitted.

**Commits.** `4f2cae4` contains the canonical spec update and archive. This
HANDOFF-only commit advances the pointer to `readme-claim-verification`.

## Completed: spec Purpose backfill gate

`spec-purpose-backfill-gate` is archived at
`openspec/changes/archive/2026-10-12-spec-purpose-backfill-gate/`; its
canonical contract is `openspec/specs/spec-purpose-integrity/spec.md`. The
local Purpose placeholder rule is enabled and its D18 code is wired into the
real Gate. The original 1,234-marker backfill and rollback manifest remain
recorded in the archived package.

The current fleet recheck finds 134 newer Purpose markers across sibling
repositories. They appeared after the original backfill and require their own
owner-authorized cross-repository cleanup. This archive record does not claim
the fleet is marker-free. A separate Driftwatchdog follow-up also remains for
the declared-code inventory weakness: the current Purpose code is gated, while
detecting future omitted codes needs the runtime's emittable-code contract.

**Verification.** Full suite — 938 tests passed, 2 skipped; workspace
self-check — 0 errors, 0 warnings; auto-gate — PASS; strict all-spec
validation — 27 passed; change names valid; diff check clean. The live fleet
backfill `--check` reports 134 later markers; the local checker and Gate
remain green for workspace-governance.

**Commits.** `eed8a6e` contains the archive and canonical spec. This
HANDOFF-only commit records the fleet limitation and advances the pointer to
`derived-kind-canonical-form`.

## Completed: local gate debt closure

`close-unbound-gate-checks` is archived at
`openspec/changes/archive/2026-10-12-close-unbound-gate-checks/`. The
repository already declared `pip-audit` in `requirements-dev.txt` and already
had a security command, but that command audited the host's global Python
installation. The Gate command now runs `pip-audit -r requirements-dev.txt`,
so it checks the dependencies this repository declares. The dependency file
itself required no change. The stale manifest note about inactive Gate
bindings was corrected.

**Gate evidence.** `pip-audit -r requirements-dev.txt` reported no known
vulnerabilities. `driftwatchdog gate` passed all six selected checks,
including `security`; no `UNBOUND` check remained. The binder dry run reports
0 unresolved checks. It preserves the scoped security command as an existing
owner command (`CONFLICT` relative to its broader generated default) instead of
overwriting it. The debt driver reports 0 entries.

**Verification.** `python3 -m unittest discover -s tests -q` — 938 passed,
2 skipped; workspace self-check — 0 errors, 0 warnings; auto-gate — PASS;
strict all-spec validation — 27 passed; change names valid; diff check clean.

**Commits.** `8737055` contains the scoped security binding, archived change,
and canonical debt-record requirement. This HANDOFF-only commit records the
Gate evidence and advances the pointer to `spec-purpose-backfill-gate`.

## Completed: declared gate command binding

`bind-declared-gate-checks` is archived at
`openspec/changes/archive/2026-10-12-bind-declared-gate-checks/`. It adds the
generator contract to `gate-command-binding` and updates the portfolio
vocabulary and product-quality requirements. The task record was reconciled
against current fleet state and links follow-up findings to their owning
changes.

The binder now labels a repository with `pyproject.toml` as Python even when
it has no test directory; a regression test reproduced `unknown` before the
fix. README documents dry-run/apply, debt reporting, repository selection,
verbose evidence, and exit codes. The grandfathering baseline was regenerated
after both the binder and debt driver measured zero outstanding debt.

**Verification.** `python3 -m unittest discover -s tests -q` — 938 tests
passed, 2 skipped; binder suite — 139 tests passed, 1 skipped; binder dry run
over 82 repositories — 0 unresolved checks, 29 existing commands left in
place, no writes; legacy debt driver — 0 entries; grandfathering baseline
check — 0 pairs, current; axis migration check clean; workspace self-check —
0 errors, 0 warnings; auto-gate — PASS; strict all-spec validation — 28
passed; change names valid; diff check clean.

**Commits.** `84483ae` contains the archive, README update, shape-reporting
fix, regression test, and zero-debt grandfathering record. This HANDOFF-only
commit records evidence and advances the pointer to
`close-unbound-gate-checks`.

## Completed: TypeScript test matcher

`anchor-typescript-test-code-match` is archived at
`openspec/changes/archive/2026-10-12-anchor-typescript-test-code-match/` and
updates the existing `Reject test code in product scope` requirement in
`openspec/specs/product-code-quality-boundary/spec.md`. The delta now retains
the Rust and cross-language scenarios alongside the TypeScript qualifier
scenarios; archive validation caught and prevented dropping those existing
scenarios.

**Verification.** `python3 -m unittest tests.test_product_code_quality -q` —
50 tests passed; workspace self-check — 0 errors, 0 warnings; auto-gate — PASS;
strict all-spec validation — 29 passed; change names valid; diff check clean.

**Commits.** `23c4f43` contains the archive and canonical requirement update.
This HANDOFF-only commit records evidence and advances the pointer to the next
incomplete change, `bind-declared-gate-checks`.

## Completed: composite gate command

`derive-composite-gate-command` is archived at
`openspec/changes/archive/2026-10-12-derive-composite-gate-command/`; its
canonical contract is in `openspec/specs/gate-command-binding/spec.md`. The
archive initially exposed malformed delta structure: two `ADDED` headings and
a `MODIFIED` requirement with no canonical baseline. The requirement is now
represented as an addition, the proposal has the required `What Changes`
section, and both command composition and ambiguity refusal are present in the
canonical spec. The change's completed task list records that the 20 remaining
binder debt entries are separate work.

**Verification.** `python3 -m unittest tests.test_bind_gate_commands -q` —
138 tests passed, 1 skipped; workspace self-check — 0 errors, 0 warnings;
auto-gate — PASS; strict all-spec validation — 30 passed; diff check clean.

**Commits.** `475e127` contains the archive and complete canonical spec. This
HANDOFF-only commit records evidence and advances the pointer to
`anchor-typescript-test-code-match`.

## Completed: scaffold gate binding

`scaffold-binds-governance-gate` is archived at
`openspec/changes/archive/2026-10-12-scaffold-binds-governance-gate/`; its
canonical requirement is in `openspec/specs/project-scaffolding/spec.md`.
The generated Purpose was replaced with the actual initializer contract before
commit.

**Verification.** Focused initializer and binder tests — 153 passed, 1
skipped; workspace self-check — 0 errors, 0 warnings; auto-gate — PASS; change
names valid; strict change validation passed; diff check clean.

**Commits.** `0434e8f` contains the archive and canonical spec. This
HANDOFF-only commit records evidence and advances the pointer to
`derive-composite-gate-command`.

## Completed: live gate debt ratchet

`ratchet-live-gate-debt` is archived at
`openspec/changes/archive/2026-10-12-ratchet-live-gate-debt/`; its canonical
requirement is in `openspec/specs/workspace-check/spec.md`. The generated
Purpose was replaced with the actual workspace-check scope before commit.

**Verification.** `python3 -m unittest tests.test_gate_debt_precondition -q` —
40 tests passed, 1 skipped; workspace self-check — 0 errors, 0 warnings;
auto-gate — PASS; change names valid; strict change validation passed; diff
check clean.

**Commits.** `7f399d7` contains the archive and canonical spec. This
HANDOFF-only commit records evidence and advances the pointer to
`scaffold-binds-governance-gate`.

## Completed: identity and taxonomy axis split

`identity-and-taxonomy-axis-split` is archived at
`openspec/changes/archive/2026-10-12-identity-and-taxonomy-axis-split/`.
It creates `openspec/specs/identity-and-taxonomy/spec.md` and updates the
existing portfolio-vocabulary and registry-integrity specs. The delta initially
combined those three targets under one spec path; splitting the modified
requirements by canonical owner let OpenSpec update all three canonical specs.
The archive also exposed two live references to the active design file; the
migration checker and its tests now read the archived design as the retained
decision table. README's taxonomy status was corrected and vocabulary docs now
link to the canonical spec.

The task record now marks the verified work complete. It explicitly records
that the original implementation commit did not preserve a separate red-test
commit for the regression tests. This historical limitation is visible rather
than claimed as TDD evidence.

**Verification.** `python3 -m unittest tests.test_identity_and_taxonomy -q` —
70 tests passed; `python3 -m unittest discover -s tests -q` — 937 tests,
passed, 2 skipped; `python3 scripts/axis_migration.py --root /home/paul/code
--governance-root . --check` — all rows applied; `workspace_check.py
--self-check` — 0 errors, 0 warnings; `auto_gate.py --self-check
--no-deployment` — PASS; `openspec validate --all --strict
--no-interactive` — 30 passed; change names valid; diff check clean. The first
full-suite run after archiving found the design-path dependency. Updating the
migration tool and test to read the archived design fixed it. The self-check
then caught the generated Purpose placeholder; the actual purpose is now in
the canonical spec.

**Commits.** `05310af` archives the change, adds canonical specs, corrects the
documentation, and updates the migration tool/tests to read the archived
decision table. This HANDOFF-only commit records the evidence and advances the
pointer to `ratchet-live-gate-debt`. Older notes below describe state at their
recorded dates; this section is the current archive state.

## Completed: legacy gate debt remediation

Changes `drive-legacy-gate-debt-to-zero` and `classify-dirty-gate-manifest-debt` are archived under `openspec/changes/archive/2026-10-12-*`. The first established the canonical `legacy-gate-debt-remediation` spec; the second extended it with reporting-only class G for dirty manifests. `openspec archive` initially rejected the second delta because the canonical spec was missing. Archiving the foundational change first restored the intended dependency, then the delta archived successfully. The canonical Purpose was filled with the actual scope before commit.

**Verification.** `python3 -m unittest tests.test_close_gate_debt -v` — 20 tests passed; `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings; `python3 scripts/auto_gate.py --self-check --no-deployment` — PASS; `node scripts/check-openspec-change-names.mjs` — valid; `openspec validate --all --strict --no-interactive` — 31 passed before archiving; `git diff --check` — clean.

**Commits.** `7ed055f` archived both dependency-linked changes and created the canonical spec; `9178469` records the evidence in HANDOFF. Pre-existing dirty template files and untracked `HANDOVER.md` and `driftwatch.toml` remain unstaged.

## Current package: restore governance verification baseline

Change `restore-governance-test-baseline`, archived 2026-10-12 at
`openspec/changes/archive/2026-10-12-restore-governance-test-baseline/`; its
canonical spec was promoted to
`openspec/specs/workspace-verification-baseline/spec.md`. Commit 1:
`47ce2f5` (`fix(governance): restore verification baseline`). This
HANDOFF-only commit completes the required pair.

**Root causes and repairs.** The active Mnemora declaration existed with
`id: mnemora`, `profile: rust-product`, `role: product`, and `stack: rust`, but
`projects.json` had no matching entry. The local registry now records that
declaration exactly; Mnemora's repository was not edited. The promotion-test
sandbox coverage list also omitted three tracked governance records:
`registry-remote-drift.json`, `tag-taxonomy.json`, and
`ui-adoption-migration.json`. The fixture now copies all three into its
temporary root. The historical 81-project taxonomy migration fixture remains
unchanged; tests record Mnemora as a later registry addition.

**Verification.** The regression tests first failed for both missing inputs.
After the repair:

| Command | Result |
|---|---|
| `python3 -m unittest tests.test_workspace_verification_baseline tests.test_identity_and_taxonomy tests.test_promote_project -v` | 94 tests, OK |
| `python3 -m unittest discover -s tests -v` | 937 tests, OK; 2 skipped |
| `python3 scripts/axis_migration.py --root /home/paul/code --governance-root . --check` | no outstanding rows |
| `python3 scripts/workspace_check.py --self-check` | errors=0 warnings=0 |
| `python3 scripts/auto_gate.py --self-check --no-deployment` | PASS, errors=0 warnings=0 |
| `node scripts/check-openspec-change-names.mjs` | valid |
| `openspec validate --all --strict --no-interactive` | 31 passed, 0 failed |
| `git diff --check` | clean |

The portfolio audit now reports 370 errors and 21 warnings. Mnemora has 23
`SPEC_PURPOSE_PLACEHOLDER` errors in its archived capability specs; those are
visible after registration and remain owned by the Mnemora repository. No
sibling files were changed. Pre-existing dirty template files and the
untracked `HANDOVER.md` and `driftwatch.toml` were preserved.

**Pointer.** `current_spec` returns to `classify-dirty-gate-manifest-debt`,
which is ✓ Complete and has no unchecked tasks. Its verification and archive
cycle is next.

## Current package: UI convention guidance and the one-repository binder

The rule existed and the checker existed; the door and the gate were missing.
Nothing in the workspace-governance skill routed an agent writing UI to
`scripts/ui_conformance.py`, and no repository Gate carried a bound
`ui-conformance` check, so the checker reported drift while blocking nothing
(ruling D18: a verifier that no gate runs does not gate). Change
`ui-convention-guidance`, completed and archived 2026-10-09 as
`openspec/changes/archive/2026-10-09-ui-convention-guidance/`, canonical spec
promoted to `openspec/specs/ui-convention-guidance/spec.md` (no `--skip-specs`;
Purpose written from the proposal, not left TBD). Two commits:
`b851cab` (route, reference, docs, binder, tests, archive, spec) then this
handoff-only commit.

**What shipped.** One UI routing row in `skills/workspace-governance/SKILL.md`
onto `skills/workspace-governance/references/ui-convention.md`; the human
mirror `docs/ui-convention.md`; and `scripts/bind_ui_gate.py`, a dry-run-default,
single-target, idempotent binder. Docs cross-check: the seven codes both files
state are exactly the seven constants at `scripts/ui_conformance.py:83-89`
(`UI_EXTENSIONS` there is an extension set, not a code), the
`UI_SCOPE_UNDECLARED` = REVIEW_REQUIRED severities match
`ui_conformance.py:690`, every `somedanote` worked-example path was verified to
exist in the sibling checkout, and `check_skill_refs.py` resolves the row.
Binder behaviour: refuses a missing manifest, a non-`workspace-governance@`
`rule_pack`, a manifest with no `checks:` block, an absolute or env-var
command, and a conflicting existing entry — in every refusal nothing is
written; a second `--apply` is byte-for-byte identical.

**Deviations found by verification, fixed before the commit.** Two:

1. `design.md` claimed in its behaviour table and contract line that the
   default dry run exits non-zero when a change is needed ("Exit zero means
   already bound"), contradicting its own CLI section, the spec — which
   mandates non-zero only for `--check` — the reference docs and the tests.
   `--check` exists precisely to carry that exit contract, so the spec's
   reading won: default previews with exit `0`; `--check` exits `1` when the
   binding is missing or wrong. `design.md` corrected to say so.
2. The spec requires the second run to report `no change`; the `ALREADY_BOUND`
   detail said only "already bound to …". The detail now carries `no change`
   and `tests/test_bind_ui_gate.py` pins it.

**Commands and results.**

| Command | Exit | Result |
|---|---|---|
| `python3 -m unittest tests.test_bind_ui_gate -v` | 0 | Ran 30 tests, OK |
| `python3 scripts/check_skill_refs.py` | 0 | skill refs ok: SKILL.md + 9 references |
| scratch dry-run/apply/second-apply sequence (fresh `/tmp` repos only; no real repository was ever passed) | 0 | default exits `0`, sha256 unchanged; `--check` unbound exits `1`; `--apply` exits `0` and writes the two entries; second `--apply` prints "already bound … no change" and leaves the sha256 unchanged; `--check` bound exits `0` |
| `python3 scripts/workspace_check.py --self-check` | 0 | Summary: errors=0 warnings=0 |
| `openspec validate --changes --strict --no-interactive` | 0 | 15 passed pre-archive, incl. `✓ change/ui-convention-guidance` |
| `openspec validate --all --strict --no-interactive` | 0 | post-archive: 30 items passed, incl. `✓ spec/ui-convention-guidance` |
| `node scripts/check-openspec-change-names.mjs` | 0 | OpenSpec change names valid |
| `git diff --check` | 0 | clean |
| `python3 -m unittest discover -s tests` | 1 | 935 tests, 2 failures, 2 skipped — the same two pre-existing failures recorded under the auth-page package above; neither names a path this change touched |

**Deliberately not run here — the follow-up action.** Binding a real
repository is the owner's decision one repository at a time; no real repository
was bound by this change. A later agent binds exactly one named repository from
this checkout:

```
python3 scripts/bind_ui_gate.py <repository-path> --check   # names what it would bind; exit 1 while unbound
python3 scripts/bind_ui_gate.py <repository-path> --apply   # writes checks.ui-conformance and commands.ui-conformance
```

The written command is `python3 ../workspace-governance/scripts/ui_conformance.py .`
and requires this checkout at its sibling path. After binding, run that
repository's own Gate once to confirm the check executes inside the gate, and
report its verdict there — binding is not conformance.

**Pointer.** `current_spec` advances to `classify-dirty-gate-manifest-debt`,
first in `openspec list` and already ✓ Complete — it needs only its
verification and archive cycle. Active changes remain, so under this
repository's one-pointer rule the line advances rather than being removed, as
it has at every prior cycle end.

**Untouched.** `HANDOVER.md` and `driftwatch.toml` stay untracked and were not
committed. No consumer repository was edited, declared or bound; the other
fourteen active changes were not modified.

## Current package: auth-page adoption evidence

`UI_AUTH_HANDROLLED` fired on a filename pattern alone and read nothing. That is
why the only repository in the workspace that adopted the shared layer failed its
own gate: `somedanote` renders `Login.cshtml` through the shared composition
carrying `platform-auth-card`, `platform-auth-title`, `platform-auth-description`,
`platform-auth-form`, `platform-auth-header`, `platform-auth-logo`,
`platform-button`, `platform-control`, `platform-shell` and `platform-state`, and
was still reported as hand-rolled. Change `ui-auth-adoption-evidence`, archived
as `2026-10-06-ui-auth-adoption-evidence`.

**What the rule does now.** It keeps the pattern, the ERROR level and the
repo-wide scope. Before reporting, it opens the file it matched and asks whether
that file demonstrates the shared layer: a `platform-*` class name, or a
stylesheet reference whose path names the shared layer — the vendored token
stylesheet or a vendored shared-layer stylesheet. A `.cshtml`/`.razor` page that
renders no markup of its own is judged by the surface it composes (`@layout`, or
the `Layout` its nearest `_ViewStart.cshtml` assigns), and only when the
repository declares adoption, the file is a Razor page, and that surface itself
shows the layer. That is how a POST-only `Logout.cshtml` is accounted for: the
sign-out control that posts to it is the shared one, rendered by the shared
shell.

**Fail-closed, itemised.** A declared adopter that hand-writes a login page is
still reported; the declaration is not consulted for any file that renders a
screen. An unreadable auth-named file is still reported. A layout that cannot be
resolved, or that shows no shared signal, is still reported. A `.tsx` file that
renders nothing is still reported — the composition gate is Razor-only. Every
file that goes unreported is written to `report.notes` naming the file and the
signal read, so a narrowed finding is visible rather than absent.

**The three-way proof**, each case a scratch repository built in
`tempfile.TemporaryDirectory()`, run through the CLI with
`--baseline /nonexistent.json`:

| Case | Result |
|---|---|
| bare `LoginPage.tsx`, no shared reference | `exit=1`, `ERROR UI_AUTH_HANDROLLED src/LoginPage.tsx` |
| same page rendering `platform-*` | `UI_AUTH_HANDROLLED` absent, note: "it shows the shared layer through a platform-* class name" |
| declares adoption, writes a fresh bare login page | `exit=1`, `ERROR UI_AUTH_HANDROLLED src/LoginPage.tsx` (the only error raised) |
| declared adopter, handler-only `Logout.cshtml` in the shared shell | `exit=0`, note names `Pages/Shared/_Layout.cshtml` |

**Fleet neutrality.** `ui_conformance.py --audit --root /home/paul/code
--baseline <none> --json` was captured before and after and every project report
compared field by field: 59 projects, exactly 2 changed, both removals.
`dotnet-platform-libs` (the provider) loses
`ui/packages/react-auth/src/auth-forms.tsx`, which is the layer's own source and
speaks the shared vocabulary; `somedanote` loses its migrated login page. Total
findings 349 → 347. No repository's findings were added, and no non-adopter
moved.

**Baseline.** `generate_ui_conformance_baseline.py --check --root /home/paul/code`
exited 1 after the change and 0 before it. The snapshot was regenerated with its
own generator: two `UI_AUTH_HANDROLLED` counts shrink by one each, nothing is
added, no code leaves any entry. It now exits 0.

**Portability.** No token semantics moved into Python. The new tests are string
shapes over file content, the same class as the existing `THEME_MECHANISM_RE`.
`UI_COLOR_LITERAL` still consults
`dotnet-platform-libs/ui/scripts/lib/token-model.mjs` over one bounded `node`
subprocess, and the run note still says so.

**Commands and results.**

| Command | Exit | Result |
|---|---|---|
| `python3 -m unittest discover -s tests` | 1 | 935 tests, 2 failures, 2 skipped — the same 2 as before this change |
| `python3 -m unittest tests.test_ui_conformance` | 0 | 60 tests (49 before) |
| `python3 scripts/ui_conformance.py --audit --root /home/paul/code --baseline <none>` | 1 | 59 projects, 347 findings, down from 349 |
| `python3 scripts/generate_ui_conformance_baseline.py --check --root /home/paul/code` | 0 | up to date after regeneration |
| `openspec validate --all --strict --no-interactive` | 0 | 30 items passed |
| `node scripts/check-openspec-change-names.mjs` | 0 | valid |
| `python3 scripts/workspace_check.py --self-check` | 0 | errors=0 warnings=0 |
| `git diff --check` | 0 | clean |

**Pre-existing failures, recorded not fixed.** `python3 -m unittest discover -s
tests` fails on two tests that pre-date this work, reproduced before the first
edit:

1. `test_identity_and_taxonomy.SequencingGateTests.test_the_gate_is_open_and_the_fleet_corpus_is_clean`
   — `refused: somedanote: design.md §5.2 rule 1: 1 records a registry profile
   but no registry entry exists`. The registry entry for that repository has id
   and path `somodanote`; the checkout declares id `somedanote`. Nothing here
   changes the registry.
2. `test_promote_project.SandboxCoverageTests.test_every_governance_root_data_file_is_accounted_for`
   — `registry-remote-drift.json`, `tag-taxonomy.json` and
   `ui-adoption-migration.json` are neither copied into the sandbox nor listed
   as absent.

**Test count correction.** `tests/test_ui_conformance.py` held **49** tests
before this change and holds **60** after it. A commit message already in this
history says "47 tests" for that file. That number was wrong when it was
written, and correcting it would require rewriting history, which this
repository does not do. The correct figure at the time that commit landed was
49.

**Not done.** No repository is migrated or declared from here; `somedanote`'s
declaration is its own change. No finding code is added, renamed or removed. The
token boundary, the scope rule and the baseline shape are untouched. The two
failures above are not fixed.

## Current package: UI convention conformance check

`dotnet-platform-libs/ui` had zero adoption across the portfolio and nothing
could see it. `scripts/product_code_quality.py` reads the `quality` block and
the `readme-status-claim` rule reads `README.md`, so a repository satisfied both
while shipping its own login form, its own colour literals and no theme
mechanism. The measured facts that motivated the change, with the surface census
fixed in the archived `design.md` §2:

| Measured fact | Value |
|---|---|
| Registered repositories | 81 |
| Repositories shipping UI surface | 59 |
| UI files across the fleet | 4280 |
| Repositories with an auth-named UI file | 22 |
| Auth-named UI files | 113 |
| Repositories declaring adoption of the shared `ui` layer | 0 |
| Repositories declaring a UI scan scope | 0 |

The eight finding codes are fixed by the portfolio owner and were implemented
unchanged: `UI_AUTH_HANDROLLED`, `UI_TOKEN_NOT_CONSUMED`, `UI_COLOR_LITERAL`,
`UI_THEME_ABSENT`, `UI_BRANDING_ABSENT`, `UI_CONTRAST_UNMEASURED` (all ERROR) and
`UI_SCOPE_UNDECLARED` (REVIEW_REQUIRED). Two things worth stating plainly
because they are design decisions rather than mechanics:

**The scope rule.** Filename-level rules run on a repo-wide census;
content-level rules read only inside the repository's declared
`ui.scope.roots`. That is what keeps `alltools-platform` — a tooling platform,
not a consumer, with 2067 UI files — bounded and visible instead of skipped or
drowning the report. It emits its 24 real auth-named file paths, its declaration
findings, and one note naming 2067 against the 400-file ceiling; it opens no
file. `dotnet-platform-libs` is recorded with `role: "provider"` because it is
the layer itself. `rust-ecommerce` (305) and `openlearning` (239) are the next
largest and sit under the ceiling.

**The token boundary.** `UI_COLOR_LITERAL` does not re-implement the resolver.
One bounded `node` process per run imports
`dotnet-platform-libs/ui/scripts/lib/token-model.mjs` and reads
`declaredLiterals(resolveAliases(readTokens(...)))`. The DTCG walk, the alias
chain and the canonical name rule stay where they belong. Every way of failing
to consult that module — no `node`, missing module, unreadable source, an
unresolved alias (which `resolveAliases` throws on), or a timeout — reports
every colour literal as unreached and exits non-zero. A literal is never
declared token-reachable by a comparison that did not run.

The baseline is the fourth piece and is not a switch: a run without
`--baseline` fails closed on every finding, a code that is not listed fails
normally, and `--check` reports a stale entry as dead weight. It records 59
projects and 258 codes across them — `UI_BRANDING_ABSENT`,
`UI_CONTRAST_UNMEASURED`, `UI_SCOPE_UNDECLARED` and `UI_TOKEN_NOT_CONSUMED` 59
times each, `UI_AUTH_HANDROLLED` on the 22 repositories that hold one.

Verification actually run, with real output:

| Command | Exit | Result |
|---|---|---|
| `python3 -m unittest tests.test_ui_conformance` | 0 | 49 tests pass |
| `python3 -m unittest discover -s tests -v` (declared) | 1 | 924 tests, **2 pre-existing failures** |
| `scripts/ui_conformance.py <conformant scratch repo>` | 0 | PASS, 1 UI file, token source compared |
| `scripts/ui_conformance.py /home/paul/code/fotofy --baseline /nonexistent.json` | 1 | 8 ERROR + 1 REVIEW_REQUIRED, 5 named auth files |
| `scripts/ui_conformance.py /home/paul/code/fotofy` | 0 | PASS with 9 findings reported as baselined |
| `scripts/ui_conformance.py /home/paul/code/alltools-platform` | 0 | PASS, 2067 files noted against the 400 ceiling, 28 baselined |
| `scripts/ui_conformance.py --audit --root /home/paul/code` | 0 | 59 projects, 0 fail, 0 review_required |
| `generate_ui_conformance_baseline.py --root /home/paul/code --check` | 0 | up to date, 59 projects, 1 over the ceiling |
| `python3 scripts/workspace_check.py --self-check` | 0 | errors=0 warnings=0 |
| `openspec validate --changes --strict` | 0 | 16/16 passed |
| `openspec validate --specs --strict` | 0 | 15/15 passed |
| `python3 scripts/check_skill_refs.py` | 0 | SKILL.md + 9 references |
| `node scripts/check-openspec-change-names.mjs` | 0 | names valid |

**Load-bearing proof, outside the test suite.** A conformant scratch repository
was built in a `tempfile.TemporaryDirectory()`, checked (exit 0), given a
hand-written `src/pages/LoginPage.tsx` (exit 1, `UI_AUTH_HANDROLLED
src/pages/LoginPage.tsx`), had the file removed (exit 0 again). The gate flips
on the thing it exists to catch. No shell delete was used anywhere; cleanup was
in-process.

Two deviations from the brief, both real and both recorded rather than papered
over:

1. **`workspace_check.py --self-check` reports 0 errors, not 3.** The three
   `SPEC_PURPOSE_PLACEHOLDER` errors named in the brief
   (`platform-build-policy`, `platform-test-web-factory`,
   `platform-ui-token-distribution`) are not present on `main` as it stands.
   Nothing was fixed to achieve that. The one placeholder this archive could
   introduce — `TBD - created by archiving change ui-convention-check` — was
   replaced with real prose, and the post-archive self-check is still 0/0.
2. **The fleet census reads 59 repositories, not 64, and 113 auth files, not
   44.** The suffix set and prune set are exactly those in the brief; 59 is what
   that set yields today. The 44 figure comes from
   `ui-adoption-migration.json`, which counted a narrower auth pattern; this
   checker's pattern includes bare `auth`, which is why it finds more.

Pre-existing failures, not introduced here and not fixed here:

- `test_identity_and_taxonomy.SequencingGateTests.test_the_gate_is_open_and_the_fleet_corpus_is_clean`
  — `axis_migration.py --check` refuses on `somodanote`, which records a
  registry profile with no registry entry. From commit `4395a3f`.
- `test_promote_project.SandboxCoverageTests.test_every_governance_root_data_file_is_accounted_for`
  — `registry-remote-drift.json`, `tag-taxonomy.json` and
  `ui-adoption-migration.json` are unaccounted. From commits `db140a6` and
  `8b5304e`. This change added only its own `ui-conformance-baseline.json` to
  the accounted set; the other three were left as found.

Commit 1 is `e07b43b` (`feat(ui-convention-check): ...`) and contains the
implementation, the tests, the baseline and the archived change
(`openspec/changes/archive/2026-10-06-ui-convention-check/` plus
`openspec/specs/ui-convention-check/spec.md`). This commit is HANDOFF evidence
only. The change is archived; the checker is standalone and is not wired into
`validate_project()`, which is a separate change in a file this one does not
own. `skills/workspace-governance/**` and `docs/ui-convention.md` belong to a
parallel agent and were not touched.

## Current package: classify dirty gate-manifest debt

The driver now recognizes the binder's dirty-manifest refusal as class G. It
reports the owner action—review and commit the manifest, then rerun the binder—
without invoking a remedy or writing the target. The matcher requires the
binder marker at the start and a non-empty derived command. Unknown reasons
remain class `?`.

The authorized full-workspace binder run refreshed `gate-debt.json` to 11
entries across 8 repositories: eight class G dirty-manifest entries and three
existing owner decisions (jenkins-local/build class C, sisyphusfy/build class F,
and sisyphusfy/security class E). No debt is claimed closed. The dry-run digest
check covered 76 sibling gate manifests and found zero changes.

The binder apply left these sibling `.ai-gate/gate.yaml` additions uncommitted;
preserve them and do not stage them from this repository:
`aegify`, `argoscope`, `cargo-audit-boost`, `cargo-matrix-boost`, `cargo-sonar`,
`dotnet-platform-libs`, `gradevia`, `kairion`, `labrys`, `metiscope`,
`openpanel`, `pluggify`, `rust-platform-libs`, `shopifireman`, `sisyphusfy`,
`smotoox`, and `testify`. Twenty dirty sibling repositories were withheld by
the binder. Five repositories had no manifest and were not modified.

Verification: focused driver tests 20 passed; full governance suite 849 passed,
1 skipped; strict OpenSpec 14/14; self-check and project governance check 0
errors/0 warnings; auto-gate PASS 0 errors/0 warnings; change names valid;
`git diff --check` clean. The read-only reviewer found no remaining issues.
The change stays active and unarchived. Commit 1 is `ab466b3`
(`feat(governance): classify dirty manifest gate debt`) and contains the
OpenSpec, driver, tests, and regenerated debt record. Commit 2 contains this
HANDOFF evidence only.

## Current package: live gate debt ratchet

`validate_new_change_with_gate_debt` now resolves current required checks and
commands using the binder's profile/manifest mirror whenever a repository has
unarchived OpenSpec work. A live unbound concern missing from `gate-debt.json`
blocks new work immediately; unreadable, malformed, or unsupported-profile
manifests fail closed. The check reads only and preserves baseline pairs,
explicit exceptions, and the named debt-closing change. A bound command does
not count as an increase, and existing snapshot debt remains sticky until the
binder regenerates its record.

The workspace-governance manifest's mandatory `governance-quality` check was
also unbound after the Driftwatchdog catch-all landed. The binder derived and
applied `python3 ../workspace-governance/scripts/workspace_check.py --root ..
--project workspace-governance --json`; the local checker and auto-gate then
passed without errors or warnings.

Verification: 44 focused debt-precondition tests passed; the complete suite ran
847 tests, 0 failures, 1 skipped. Strict OpenSpec validation passed 13/13;
workspace self-check and auto-gate passed with 0 errors and 0 warnings; change
names and `git diff --check` passed. The live-clean/live-unbound integration
fixture confirms the next new change fails and names `late-check`. The change
remains active and is not archived. Commit 1: `8ab77e6`
(`feat(governance): ratchet live gate debt before new changes`). Commit 2 records
this HANDOFF update only.

## Current package: new-project gate binding

`init_project.py` now rejects profiles absent from the binder registry before
creating the target, adds the ten Driftwatchdog workspace profiles to the
binder's profile mirror, and invokes the binder's repository-scoped apply API
after creating both `.project.json` and `.ai-gate/gate.yaml`. Only a manifest
and declaration created by this initializer invocation bypass the binder's
clean-tree guard. A partial result reports each unresolved check and leaves the
scaffold blocked; binder refusal fails initialization and preserves files.
The scoped run does not rewrite the workspace-wide debt snapshot.

Verification: focused initializer/binder tests ran 153 tests, 1 skipped;
`python3 -m unittest discover -s tests -q` ran 839 tests, 0 failures, 1
skipped. Strict OpenSpec validation passed 12/12; workspace self-check reported
0 errors and 0 warnings; auto-gate passed with 0 errors and 0 warnings; change
names and `git diff --check` passed. A real temporary scaffold invocation
bound the repository command, reported four unresolved checks, exited 0 for
the completed bind operation, and did not write a partial workspace debt file.
The change stays active and must not be archived without authorization. Commit 1:
`e17615d` (`feat(governance): bind gate checks during project scaffold`).
Commit 2 records this HANDOFF update only.

**Legacy debt repair is implemented and verified, but not archived.** The
original 24-entry snapshot classified A=10, B=7, C=3, D=2, E=1, F=1 with no
unknown reasons. The driver and binder fixes now verify both `.sln` and `.slnx`
project membership, derive Python builds only from `[project]`, `[build-system]`
and an explicit `build` frontend, and preserve debt when a dirty manifest
blocks a write. The current measured debt record has 5 checks in 3 repositories:
`jenkins-local/build` (no package metadata), `sisyphusfy/build` and
`sisyphusfy/security` (owner decisions), and `hestiaLab/build` plus
`hestiaLab/security` (manifest is dirty; derived commands remain unbound).

Verification: `python3 -m unittest discover -s tests -q` ran 838 tests, all
passed; `openspec validate --changes --strict` passed 11/11; workspace
self-check passed with 0 errors and 0 warnings; auto-gate passed; OpenSpec
change names and `git diff --check` passed. The three existing solution files
contain all projects: aegify 2/2, hestiaLab 2/2, smotoox 4/4.

Live checker results: Mewo accessibility fails at
`apps/mewo_mobile/web/index.html:1` because `<html>` has no `lang`; Actoria and
trailCrew provenance fail because there is no `THIRD-PARTY-NOTICES`; sampled
privacy checkers pass; platform-contracts fixtures and schemas pass. No product
documentation was edited to suppress findings. Alethefy produced an sdist and
wheel successfully with `uv run --with build python -m build` (artifacts were
written under `/tmp`). Its exact gate command assumes the Python build frontend
is installed in the gate execution environment; the host's system `python3`
does not provide it. Python dependency audits could not upgrade pip because
the index/network failed, so the existing `pip-audit` declarations remain bound
but runtime audit success is unverified.

The debt driver still records five unresolved checks in three repositories;
two are owner decisions, two are withheld because HestiaLab's manifest is
dirty, and one lacks Jenkins-local package metadata. Do not fabricate bindings
or edit those repositories to force a zero count.
Commit 1: `af095e8` (`feat(governance): drive legacy gate debt repairs`). Commit 2 records this handoff update. The change remains unarchived; archiving needs separate authorization.

**A cross-cutting shared capability is now declarable in every profile.**
`capability-adoption-gate` measures adoption as a three-way join and expects a
consumer to record the pairing in `capabilities[<id>].consumes[]`, but
`validate_capabilities` warned `CAPABILITY_UNSUPPORTED` for any declared id its
profile did not list — and no profile listed the registry's shared ids. A
consumer that recorded a lawful, evidenced adoption of `telemetry`,
`rate-limiting`, `idempotency`, or `feature-flags` was therefore reported as an
anomaly.

Reproduced before the fix, by calling the checker directly: a `dotnet-product`
declaration recording `capabilities.telemetry` produced exactly one finding,
`WARNING CAPABILITY_UNSUPPORTED capability 'telemetry' is not declared as
supported by the project profile`.

Fixed at `22638ae` by adding the four **cross-cutting** shared ids to the
`supported_capabilities` list of all twelve profiles that declare one. The
checker is unchanged and keeps its purpose: the registry's role-shaped ids
(`persistence-postgres`, `health-checks`, `web-application-factory`,
`postgres-test-fixture`) stay out by decision, the profile list stays an
intentional maintainer statement, and the negative control (`billing` under
`dotnet-library`) still warns. Three tests pin all of it, including the
reproduction now green and the negative control still red.

The change is archived (`2026-10-05-admit-cross-cutting-capabilities`) and the
canonical `platform-capability-manifest` requirement it amended already carried
a real `## Purpose`, so no archiver `TBD` marker was introduced. This unblocks
the first real consumer: `argoscope` may adopt
`Platform.Observability`/`Platform.Web.Telemetry` and record the required
`consumes[]` edge without the spurious warning.

**The TypeScript test-code rule was matching a name and ignoring the
qualifier.** Measured against the real repositories: **15 findings in 4
repositories, and every one was a false positive.**

| Repository | Findings | What they actually were |
|---|---|---|
| `argoset` | 2 | a regex literal; a doc comment naming a `*.test.ts` file |
| `crossify` | 1 | `/attestation/i.test(err.message)` |
| `fotofy` | 6 | regex literals matched against a user-agent string |
| `transitfy` | 6 | compiled-regex `.test()`, a scheme regex, a whitespace regex |

A rule that reports 15 false positives and no true ones is not protecting
anything; it is only making four repositories red.

The information needed to be correct was in the line and was discarded. The
discriminator is the character before the name:

| Line | Before the name | What it is |
|---|---|---|
| `test('x', ...)` | start, or whitespace | the framework global |
| `/re/.test(v)` | `.` | a regular expression |
| `PATTERN.test(v)` | `.` | a compiled `RegExp` |
| `"s".test(v)` | `.` | a `String` method |
| `contest(1)` | a word character | a different identifier entirely |

So the match now rejects a preceding dot or word character, and strips member
calls *before* the final decision — which is what makes a line carrying both a
member call and a bare call still report, because the bare call is the test. A
comment line is excluded before matching: a comment cannot call anything, and
the remediation this rule exists to give makes no sense for prose.

| Measurement | Before | After |
|---|---|---|
| `product-test-code` in the 4 repositories | 15 | **0** |
| `product-test-code` fleet-wide | 15 | **1** |
| `unittest discover -s tests` | 810 passed | **812 passed, 0 failed** |

The one remaining finding is a genuine `#[test]` in `alltools-platform`'s Rust
source — not a TypeScript artefact, and not fixed here.

**No exclude list. No filename list. No language turned off. No threshold.** The
same approach the Rust module-tree fix used, and the same refusal: an exclude
list encodes today's false positives and silently accepts tomorrow's. The
negative control is what keeps the rule alive — if the qualifier anchor were
dropped to silence these, `test_a_bare_global_call_is_still_reported` goes red
instead of the rule dying quietly.

### A polyglot repository no longer gets refused for having two halves

24 required checks across 11 repositories were unbound for one reason: the binder
found two equally plausible entry points and, having only two answers — bind
one, or refuse — refused. The rule behind that is sound (*a wrong command is
worse than a listed gap*) and it stays. What changed is that there is now a
third answer.

`crossify` holds `backend/CourseBridge.slnx` and `frontend/package.json`. Both
answer `build`. They are not two answers to one question — they are two halves
of one repository, and the command that builds the repository is both:

```
(dotnet build backend/CourseBridge.slnx) && (cd frontend && npm run build)
```

The test for composability is **different stacks**, not "two candidates". Two
Flutter packages are one product described twice; composing them would run the
same suite twice, and picking one would skip the other. So same-stack ambiguity
still refuses, naming both, and `AmbiguityTests` now proves both halves of that.

| Measurement | Before | After |
|---|---|---|
| Unbound checks | 44 | **24** |
| Repositories carrying debt | 22 | **12** |
| `unittest discover -s tests` | 810 passed | **812 passed, 0 failed** |
| `workspace_check.py --self-check` | exit 0 | **exit 0** |
| `openspec validate --all --strict` | 23 passed | **24 passed, 0 failed** |

### The bug this change nearly shipped

Each part is wrapped in a subshell, and that is not decoration. A candidate may
`cd` into its own directory, and `cd` is not scoped to one command in an `&&`
chain. `alltools-platform` would have received:

```
cd rust && cargo build --workspace && cd django_archived && npm run build
```

The second `cd` runs from inside `rust/`, where `django_archived` does not
exist. Measured in a real shell before the apply: flat exits 2, wrapped exits 0,
same directories. Ten manifests were written after that measurement, not before
it. `test_each_part_of_a_composite_runs_in_its_own_directory` runs both forms
against real directories, because a string assertion cannot see a shell
semantics bug.

### A wrong fix, reverted

The debt record was being written under the workspace root rather than the
governance root, so regenerating it looked successful and left the
authoritative copy stale. The first fix hard-coded the script's own repository
as the default — and was wrong: a default that ignores the `--root` the caller
passed is not a default, and the first test run **overwrote this repository's
real `gate-debt.json`** with one test repository's debt. Reverted. The record
follows `--root`; `--debt-out` redirects it. `GateOracleTest` passing is the
evidence it is right again, and two tests now pin the contract — one of which
asserts the real file was not touched.

### The debt this repository was carrying

`QUALITY_NEW_CHANGE_WITH_GATE_DEBT` then refused to let this change open, because
`security` was unbound here. That is the rule working on the repository that
wrote it, and the response was to close the debt rather than waive it: the tests
imported `jsonschema` and `PyYAML` with nothing declaring them, so
`requirements-dev.txt` now declares them plus `pip-audit`. Run for real before
binding — no known vulnerabilities, exit 0.

### What is still unbound, and why

24 checks in 12 repositories. Each is real work or an owner decision, and each
carries its reason in `gate-debt.json`:

| Reason | What closing it needs |
|---|---|
| no `scripts/check-*.py` to run | a real checker, or `checks: <concern>: false` with a reason |
| a pure-Python package has no build step | a packaging decision by the owner |
| several `.csproj` and no solution | a `.sln` in the repository |
| no Python security tool declared | declaring one, as done here |
| two same-stack entry points | an owner decision on which is the product |

`opendockify` is `REVIEW_REQUIRED` on `readme-claim-unscoped`. Its README line
denies the e-signature seam in the MVP, and names no surface the checker can
resolve. The README is accurate; the checker's surface vocabulary is what is
narrow. It is not fixed here, and no repository's documentation was edited to
make a check pass.

**The test suite is green, and one decided row was the reason it was not.**
`test_the_gate_is_open_and_the_fleet_corpus_is_clean` was the only failure
across 761 tests. It runs `axis_migration.py --check` over the live 81-project
fleet, and that refused:

```
refused: lileililiwen-site: design.md §5.2 rule 4: ... records kind 'product'
but the declaration says 'deployment'; the fleet has drifted from the decided table
```

**The guard was right; the table was stale.** The owner corrected
`lileililiwen-site` to `stack: none` on two surfaces on 2026-10-05 — the sibling
declaration in `e9cc5df` ("a static site ships no implementation language", and
"Python runs only at build time for the content validators") and the registry in
`9bab1b7` ("The project's own `.project.json` already declares stack 'none', so
the two disagreed and `workspace_check` reported `TAXONOMY_MISMATCH`"). The
third surface, the §5.2 decided table, was not updated, so it still described a
superseded decision.

The mechanism is the guard's accepted set, not a code defect. Under ruling 7 a
decided row accepts its own recorded `kind` **and** the derived value of its own
`role` × `stack`. With `stacks: ['python']` that set is
`{'product', 'python-deployment'}`, while `role: deployment` with
`stack: none` derives the bare `'deployment'` under ruling 5. A value that is
correct on both live surfaces was therefore refused against its own table. The
fix is the data, not the guard: the §5.2 `stack` cell is now `none`, and the
guard accepts the bare derived role without being loosened.

The extent was measured before the edit, because `_assert_row_matches_fleet`
raises on the **first** refusal and hides everything behind it. A read-only
probe of every decided row against the live declaration and registry axes found
exactly **2** divergences, both this project's `stack`, and no other row.

Also changed, and why it had to change with the table:

- `migration/identity-and-taxonomy-axis-split.json`, the two
  `lileililiwen-site` rows: `stack`, `rule` and `source`.
  `declared_kind` stays `product` on purpose. `--write-plan` would regenerate
  the file from today's fleet and rewrite `declared_kind` to the post-migration
  `deployment` on all 81 rows, destroying the pre-migration measurement ruling 6
  exists to preserve, so the four changed fields were edited directly.
- `tests/test_identity_and_taxonomy.py`: the existing pin on this row moves from
  `"python"` to `"none"` — still an exact assertion, so drift off `none` still
  fails. Three tests were added, because the break had no cheap guard:
  `test_a_stack_none_row_accepts_the_bare_derived_role`,
  `test_a_stack_none_row_still_refuses_a_third_kind` (`kind: inventory` is still
  refused, so ruling 9 is not a blanket pass), and
  `test_the_decided_table_agrees_with_the_live_fleet_on_both_axes`, which pins
  the invariant whose absence let a clean corpus fail.
- **Negative control, run rather than asserted:** reverting only the §5.2
  `stack` cell turned **4** tests red. `design.md` was restored and verified
  byte-identical with `sha256sum -c`.
- Two scenarios added to `specs/identity-and-taxonomy/spec.md`, and §10 ruling 9
  records the correction and supersedes only the stack half of ruling 4. Ruling
  4's `role: deployment` is unchanged, and the census is re-measured at
  `python: 16`, `typescript: 1` — it grew from 12 because the validator set grew,
  and the conclusion does not change with it.

**This change is implemented and still not archived**, as are seven others:
`bind-declared-gate-checks`, `capability-adoption-gate`,
`close-unbound-gate-checks`, `cross-repo-adoption-grant`,
`derived-kind-canonical-form`, `readme-claim-verification`,
`spec-purpose-backfill-gate`. Recorded here rather than archived, because
archiving is a separate decision from fixing the failures, and because
`spec-purpose-backfill-gate` is deliberately unarchived while its gate is not
wired (D18). **`bind-declared-verification` is no longer among them.** The
untracked directory held only the `--apply` run's `rollback-manifest.json` and
no proposal, design, tasks or spec delta, so it was never a change; it was the
one item `openspec validate --all --strict` reported as failing. The change
itself is **`bind-declared-verification-command` in `driftwatchdog`**, which
owns the Gate contract its two spec deltas amend
(`gate-project-configuration`, `generic-gate-contract`), and it is valid under
`--strict`. The tool lives here, the contract lives there. The record is
tracked at `migration/bind-declared-verification-command-rollback.json` and the
empty directory is gone, so `openspec validate --all --strict` is **22 passed,
0 failed**.

**The 8 failures are fixed, and the fixture that caused 7 of them no longer
breaks that way.** `Ran 800 tests, FAILED (failures=8)` — 7 in
`test_promote_project`, 1 in `test_gate_debt_precondition` — is now **803
passed, 0 failed**, three consecutive runs. The cause was as recorded: the new
committed input `gate-debt-baseline.json` was not in
`test_promote_project.setup_sandboxed_governance_root`'s copy list, so the rule
failed closed on the sandbox and `promote_project` returned 2. `9105360` adds
the three records, and replaces the hand-kept list itself with one declared
set plus an exclusion table that states its evidence, and a test that fails
when a new top-level `.json` reaches the root and is in neither. Measured
negative control: a probe file at the root turns that one test red naming the
file, where the old fixture surfaced the same gap as 7 unrelated exit-code
failures. The exit-code assertions were checked against `promote_project`'s own
contract, not against current behaviour: two codes exist, `EXIT_OK = 0` and
`EXIT_REJECTED = 2`.

The prior active change, `derived-kind-canonical-form`, is implemented and
committed at `97f6c68`, **not archived**. It is a one-rule
checker fix: a `kind` outside the frozen 12-value `kinds` vocabulary is now
known when it is the value its own canonical `role`/`stack` derive, so the 17
`KIND_UNKNOWN` warnings the fleet audit reported — every one of them a correct
value — are gone. The four ways of *not* being self-consistent still report at
WARNING, `kinds` stays 12, and no declaration and no sibling repository was
written. See `openspec/changes/derived-kind-canonical-form/` and the evidence
entry below.

The `standalone-project-boundary` change is implemented and archived in
commit `fb5ff5c`. Workspace Governance now ships a fail-closed
standalone-readiness contract: a project may declare a `standalone`
block and an `integrations` array in `.project.json`, and the audit
classifies it as one of six canonical statuses — `standalone`,
`standalone-with-optional-integrations`, `library-dependent`,
`workspace-dependent`, `service-dependent`, `blocked` — without
executing any project command. The Forge governance adapter exposes
the same classification as four additive observation fields
(`standalone_status`, `standalone_observed_revision`,
`standalone_integrations`, `standalone_findings`); existing keys and
exit codes are unchanged. The schema, vocabulary, audit, and adapter
all stay backward compatible: existing declarations without the new
blocks remain valid and report `blocked` with a `STANDALONE_BLOCKED`
warning.

The `shared-postgres-runtime-profile` change is implemented and archived in
commit `d5508cc`. Workspace Governance now ships four `*-postgres-external`
runtime template families (`dotnet-web-postgres-external`,
`rust-web-postgres-external`, `python-web-postgres-external`,
`worker-postgres-external`) that connect an app container to a
supplied PostgreSQL service without starting a database container or volume.
`scripts/runtime_templates.py` now supports `optional_variables`, refuses a
conflicting `!.env*` unignore rule with `SECRET_IGNORE_CONFLICT` before any
write, idempotently amends the root `.gitignore` to ignore `.env`, writes a
blank-password `.env.example`, and records `database_source: external` in the
project declaration. The existing six per-project PostgreSQL container
templates are byte-identical and unchanged.

The `toolchain-decision-register` change is implemented and archived in
commit `27282ee`. Workspace Governance now ships `decisions.json` at the
governance root, a fail-closed `scripts/decisions.py` loader, and a
`workspace_check.py --decisions` summary path. The four toolchain
decisions resolved on 2026-09-30 are seeded with their owner projects,
downstream references, and policy-boundary notes; the D2 public-catalog
exclusion and the D3 capability-registry split are machine-checked by
`tests/test_decisions.py`.

The `repository-discoverability-governance` change is implemented and archived
in commit `1c434b5`. Workspace Governance now audits standard adopted projects
for `LICENSE`, source-backed `.project.json.discoverability`, and separate
GitHub publication evidence. `scripts/init_project.py` performs additive local
repair, while `scripts/github_metadata.py` requires authenticated `gh`,
preserves existing remotes/topics, and supports explicit check/apply/publish
modes without token handling or force-pushes.

The `runtime-template-library-and-renderer` change is implemented and archived
in commit `138844d`. It adds the reusable runtime source library under
`templates/runtime`, six typed Dockerfile/Compose families, and
`scripts/runtime_templates.py` with list/inspect/render/check operations.
Rendering is repository-relative, secret-free, atomic per output, and refuses
existing files unless explicitly named with `--replace`. It records configured
template metadata without changing deployment promotion state. No product
repository, Jenkins consumer, or DriftWatchdog implementation was changed.


The `work-on-main-baseline-rule` change is implemented and archived in
commit `bb4c406` as `2026-10-03-work-on-main-baseline-rule`, and the
canonical spec is `openspec/specs/rules-baseline/spec.md` (+4 added, 2
modified, 0 removed). An agent must now not create a branch or worktree to
do work **unless the user asks for one or designates one**. The rule is one
sentence, byte-identical in `templates/project/.ai-rules/workflow.md` (the
canonical source `init_project.py` copies), `templates/project/AGENTS.md`,
this repository's own `.ai-rules/workflow.md` and `AGENTS.md`, and the
global skill. It is machine-reported as the `work-on-main rule` clause of
`scripts/rules_drift.py`, which moves the fleet report from
`drift_findings=80` to `148` — a delta of exactly 68, none for this
repository.

**That clause enforces nothing, and the spec says so.** `rules_drift.py` is
wired into no gate: `workspace_check.py` imports only `scaffold_handoff`,
and the clause report is invoked by neither `workspace_check.py`,
`auto_gate.py`, nor CI. Under ruling D18, a verifier that no gate runs does
not gate. Wiring it is decision `D5` and the `rules-drift-gate-wiring`
follow-up in `ROADMAP.md`, deliberately a separate change because it turns a
148-finding triage report into a fleet-wide gate in one step. Until it
lands, the work-on-`main` rule is **stated and reported, not enforced**.

## Latest verification evidence

- `admit-cross-cutting-capabilities` (`22638ae`) evidence. The defect was a
  data mismatch between profiles written before the capability registry and the
  registry's shared ids, not a missing check. Fixed by adding `telemetry`,
  `rate-limiting`, `idempotency`, `feature-flags` to `supported_capabilities`
  in `dotnet-library`, `dotnet-product`, `governance`, `platform`, `product`,
  `python-product`, `python-runtime`, `rust-platform`, `rust-product`,
  `tooling`, `typescript-monorepo`, `typescript-product`; `scripts/workspace_check.py`
  and `capabilities.json` are untouched.

  | Gate | Result |
  |---|---|
  | `python3 -m unittest discover -s tests` | **815 passed, 1 failed** — 812 prior + 3 new; the one failure is the pre-existing `test_bind_gate_commands.GateOracleTest.test_a_fresh_run_reproduces_the_gate_set` fleet gate-debt drift, unchanged by this change and naming no `argoscope` row |
  | `python3 scripts/workspace_check.py --self-check` | `errors=0 warnings=0`, exit 0 |
  | `python3 scripts/auto_gate.py --self-check --no-deployment` | `PASS errors=0 warnings=0` |
  | `openspec validate admit-cross-cutting-capabilities --strict` | valid (pre-archive) |
  | `openspec validate --all --strict --no-interactive` | **24 passed, 1 failed** — the failure is the untracked `drive-legacy-gate-debt-to-zero`, unchanged from baseline |
  | `node scripts/check-openspec-change-names.mjs` | change names valid |
  | `git diff --check` | clean |

  The three new tests are the reproduction (green), the profile sweep (every
  profile with a `supported_capabilities` list carries all four ids, failing
  naming the profile and id), and the **negative control** (`billing` under
  `dotnet-library` still emits `CAPABILITY_UNSUPPORTED`). The checker's emitted
  finding set is otherwise unchanged: the only difference is that the four ids
  no longer warn when declared. No sibling repository was written and the
  pre-existing untracked work in this tree (`driftwatch.toml`,
  `scripts/check_concern.py`, `scripts/close_gate_debt.py`,
  `openspec/changes/drive-legacy-gate-debt-to-zero/`) is not staged.

- **Fixture and strict-validation cleanup (`9105360`).** Two problems cleared,
  both diagnosed before acting and neither papered over.

  | Gate | Result |
  |---|---|
  | `python3 -m unittest discover -s tests` | **803 passed, 0 failed**, 3 consecutive runs |
  | `python3 scripts/workspace_check.py --self-check` | `errors=0 warnings=0`, exit 0 |
  | `python3 scripts/axis_migration.py --root /home/paul/code --check` | exit 0 |
  | `python3 scripts/workspace_check.py --root /home/paul/code --audit` | `Summary: errors=279 warnings=20` — the pre-existing fleet baseline; **0** are `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` |
  | `openspec validate --all --strict --no-interactive` | **22 passed, 0 failed** |
  | `node scripts/check-openspec-change-names.mjs` | change names valid |
  | `git diff --check` | clean |

  **The 7 `test_promote_project` failures: cause confirmed, then the list
  itself fixed.** The diagnosis held — the new committed input
  `gate-debt-baseline.json` was absent from the sandbox, the
  `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` precondition failed closed, and
  `promote_project` returned 2. What was wrong with the documented remedy is
  that it extends a hand-kept list that had **already broken twice on this
  exact axis** (`rules-drift-baseline.json`, then these three), spread over
  three ad-hoc blocks and guarded by `if source.is_file()` — under which a
  file that failed to copy is indistinguishable from one never needed, exactly
  the "checker that reports OK without comparing" pattern `AGENTS.md` calls
  worse than no checker. Replaced with one declared set
  (`GOVERNANCE_ROOT_DATA_FILES`, six files, copied unconditionally, a missing
  declared file raising), a table of exclusions each stating its evidence, and
  `SandboxCoverageTests` (3 tests) failing when a top-level `.json` reaches the
  root and is in neither. `projects.json` is excluded because the fixture writes
  its own registry; `decisions.json` because a portfolio audit tolerates its
  absence with a single `DECISIONS_REGISTER_ABSENT` warning
  (`load_decisions_register`) and only `--self-check` requires it. **Negative
  control, run:** a probe `gate-debt-probe.json` at the root turns that one
  test red naming the file and the remedy, where the old fixture surfaced the
  identical gap as 7 unrelated exit-code failures; the probe was removed and
  the suite returned to green. The `SandboxCoverageTests` absence-reason
  assertion also caught a real defect in the new table on its first run — a
  reason with no terminal punctuation — which is why the table reads the way it
  does.

  **The exit codes are the intended ones, checked against the contract and not
  against current behaviour.** `promote_project` defines exactly two,
  `EXIT_OK = 0` and `EXIT_REJECTED = 2` (`scripts/promote_project.py:34-35`).
  The tests assert 0 for a completed promotion, a completed demote and the
  idempotent re-run, and 2 for each refusal — missing evidence ref, conflict
  without `--force`, unknown project id. A code of 3 appears nowhere in the
  module. The assertions describe the contract.

  **`bind-declared-verification` was a stray, not unfinished work.** Evidence,
  in order of strength: (1) its `rollback-manifest.json` is the only one of the
  three in this repository keyed on `"manifest": ".ai-gate/gate.yaml"` — the
  file `scripts/bind_verification_command.py` writes — and all 73 entries have
  differing before/after SHAs, so it is a real fleet write record, not a
  placeholder; (2) for `forge`, the manifest's `5019d129` → `19a623fa` pair
  diffs to **exactly** the `declared-verification` entry plus its generated
  comment, and the second commit is titled `bind(gate): run this repository's
  declared verification command`; (3) the binder's own module docstring cites
  `driftwatchdog/openspec/changes/bind-declared-verification-command/design.md`
  §1, and that change exists with `proposal.md`, `design.md`, `tasks.md` and
  **two** spec deltas (`gate-project-configuration`, `generic-gate-contract`),
  and is valid under `--strict`. The tool lives in this repository and the
  Gate contract it writes lives in the sibling that owns those capabilities, so
  the change was never going to be authored here under that name. The record is
  tracked at `migration/bind-declared-verification-command-rollback.json` —
  untracked it was one `git clean` from gone, and it is the only rollback
  record for a 73-repository cross-repo write. The empty directory was removed
  with the recoverable-delete tool, not `rm`. No sibling repository was
  written, read-only checks only.

- `bind-declared-gate-checks` (`8e3aecd`) evidence. **This change was resumed,
  not started**: the generator and the spec directory existed uncommitted from an
  aborted session, and the gap was that a 47KB tool with no tests is exactly the
  thing that quietly rots. `tests/test_bind_gate_commands.py` now holds **92
  tests** over `tmp_path`-sized fixture repositories, with one tolerant walk of
  the real 81.

  The tool binds each gate check a repository *already declares* under
  `.ai-gate/gate.yaml` `checks:` to a command derived from what is on disk, and
  refuses to invent one. Measured fleet dry run, unchanged before and after the
  tests:

  | | |
  |---|---|
  | repositories | 81 |
  | RESOLVED | 51 |
  | PARTIAL | 25 |
  | UNRESOLVED | 0 |
  | refused | 0 |
  | NO_MANIFEST | 5 — `chronicleaf` `devloom` `kairovia` `lileililiwen.github.io` `ziban` |
  | unresolved checks | 48 |
  | left in place | 18 |
  | debt record | `<root>/gate-debt.json`, written only under `--apply` |

  The refusals are pinned harder than the successes, because a binder that
  fabricates a command is worse than no binder. **The negative control was run,
  not asserted.** Making `derive` return `python3 -m build` for a pure-Python
  package turned three tests red —
  `AssertionError: 'BOUND' != 'UNRESOLVED'` and
  `AssertionError: ' -m build' unexpectedly found in 'python3 -m build'` — and
  the binder was restored byte-identical before committing.

  Two field errors are locked in by name. **19 repositories use `*.slnx`, not
  `*.sln`**; `SOLUTION_SUFFIXES` is asserted to be `(".sln", ".slnx")` and a
  `.slnx`-only fixture must still produce `dotnet build Foo.slnx`. And
  **`loreMix` is `LoreMix.slnx` while labelled `typescript`** — a label-driven
  binder emits `npm run build` for it. The declared `stack` is therefore not an
  argument of any function in the tool. `FleetTest` walks the real workspace once
  and asserts invariants only, never a count, so it survives fleet churn; one of
  its assertions is that every path a bound command names actually exists in
  that repository.

  Suite: **615 → 707 passed, 155 → 158 subtests passed.** No test was weakened,
  skipped or filtered.

  Two reporting quirks the tests found, reported and **not** fixed, because
  fixing them means editing the generator rather than adding tests. A Python
  repository with only a `pyproject.toml` and no `tests/` directory is labelled
  `unknown` in the debt record's `shape` field, because `Shape.label()` reads
  `python_dirs`, which detection populates only from a test directory. And a
  second `--apply` immediately after the first sees its own writes as `DIRTY`,
  skips those repositories, and so writes a debt record that under-reports —
  two *dry* runs are byte-identical, which is what the tests assert.

  `scripts/bind_verification_command.py` changed only by the two-helper rename
  the generator needs to reuse its YAML editor. The sibling's own tests pass and
  its dry run still reports `BOUND: 0, ALREADY_BOUND: 76, CONFLICT: 0,
  REFUSED: 0`, so no behaviour moved.

  `openspec validate --changes --strict` reports this change valid, and
  `openspec validate --all --strict` is **22 passed, 0 failed**. The tree's
  former one failure, `bind-declared-verification`, was an untracked directory
  holding no `specs/` delta at all — a stray copy of the verification binder's
  `--apply` rollback record. That change is
  `bind-declared-verification-command` in `driftwatchdog`, which owns the Gate
  contract; the record is tracked here at
  `migration/bind-declared-verification-command-rollback.json`. Resolved at
  `9105360`.

- `spec-purpose-backfill-gate` (`6de0dd7`) evidence. Archiving a change writes
  `TBD - created by archiving change <name>.` into the `## Purpose` of every spec
  it produced, so a spec whose purpose is the string TBD reads as a finished
  spec. Measured 2026-10-03: 79 repositories carry an `openspec/specs` tree
  holding **1,958** spec files, of which **1,234** across **71** repositories
  still hold the marker — 1,229 in the current form with the trailing
  `Update Purpose after archive.` sentence, 5 in the earlier short form, and 7
  of them with a blank line between the heading and the marker. The archiver is
  third-party (`@fission-ai/openspec`), so the defect is held here.

  `SPEC_PURPOSE_PLACEHOLDER` is **ERROR** and names the spec file relative to
  its repository plus the change the marker records, so the remediation is
  "resolve this spec from this change". It is its own finding-code family, is
  skipped for a repository with no specs tree, and is deliberately not gated on
  `profile_requirements`: a descriptive rule must not become a requirements
  driver.

  The rule is **fenced off the fleet** by `SPEC_PURPOSE_AUDIT_ENABLED = False`,
  a committed repo-relative constant in `scripts/workspace_check.py` following
  the `TAXONOMY_AUDIT_ENABLED` pattern exactly, and it is the only hold — no
  environment variable, no skip flag, no exemption registry. The severity was
  not softened to WARNING to achieve it. Both states are measured on the real
  fleet, not only on a fixture: the audit reports **205 errors** with the fence
  closed and **1,439** with it open, which is 205 plus exactly the 1,234
  markers. The constant is committed back at `False`.

  `scripts/spec_purpose_backfill.py` is the shared owner of the remediation: a
  dry run is its only default, `--apply` is its only write path and is
  all-or-nothing, and every plan row records the source that resolved it. It
  prefers the archived change's own `## Why` prose — the marker holds the **bare**
  change name while the archive directory is **date-prefixed**, so the two are
  joined by stripping `YYYY-MM-DD-`, never by concatenation, which resolves zero
  files — and falls back to a requirement-derived sentence preserved
  byte-for-byte from the `openlearning` prior art. A change name matching two
  archive directories is refused **row-level**, so the other 1,232 rows stay
  reviewable; 1,089 clear the recorded 120-character `## Why` floor.

  Two defects in the recovered `ff33661` checkpoint were fixed. `--check`
  applied the drift refusal, so on a fully applied fleet — where every planned
  marker is legitimately absent — it exited 1 in all three states and could never
  report "applied"; `build_plan` now takes `verify_committed` and `--check`
  passes `False`, so not-applied is 3, applied is 0, and re-applied is
  idempotent at 0. And the marker pattern matched any line *starting* with the
  marker, so a real sentence appended after it was a finding, contradicting its
  own comment and the spec delta; the marker must now be the whole `## Purpose`
  section, with the change name bound to `[\w.-]+` — which a change name
  satisfies and prose cannot. Measured, this matches the same 1,234 fleet files
  and no others.

  **D18 is not satisfied, and this is recorded as a blocker rather than claimed
  as wiring.** The `governance-quality` concern in `driftwatchdog` filters its
  verdict to the closed allow-list `GOVERNANCE_QUALITY_DECLARED_CODES`, which
  holds four codes and excludes this one, and it treats the checker's exit 1 as
  an *expected* exit. With the new-code findings filtered out the concern yields
  `PASS`, so enabling this rule today would turn this repository's CI red while
  the fleet gate reported green. Adding `"SPEC_PURPOSE_PLACEHOLDER"` to that set
  is a separately authorized cross-repo step in a repository this change does not
  own, and `SPEC_PURPOSE_AUDIT_ENABLED` must stay `False` until it lands. The
  change is committed **unarchived** for the same reason: archiving would claim a
  finished gate that does not exist.

  52 new tests. The suite is **553 passing** from 501, with no existing
  assertion weakened; `--self-check` is 0 errors / 0 warnings; the fleet audit
  is unchanged at **205 errors / 18 warnings** with the fence closed;
  `openspec validate spec-purpose-backfill-gate --strict` passes. The 1,234-file
  fleet write is **not applied** — it is a cross-repo source edit requiring
  explicit authorization, exactly like the axis migration — and the 2 ambiguous
  rows are left for an owner to decide by hand.

- `derived-kind-canonical-form` (`97f6c68`) evidence. This is a **checker**
  defect, not 17 portfolio defects: every one of the 17 `KIND_UNKNOWN`
  warnings the fleet audit reported was a *correct* value.
  `identity-and-taxonomy-axis-split` made `kind` a derived, deprecated
  composite, so its authority is the `role`/`stack` pair — and the derivation
  is a product of two enumerated axis sets, 6 × 6 = **36** values, of which
  **27 are absent** from the deliberately frozen 12-value `kinds` list. A
  value that is correct by the current rule is therefore often not in the list,
  so holding a derived value to the list reported a defect no owner could act
  on. `forge` (`rust-platform`), `driftwatchdog` (`rust-tooling`) and this
  repository (`python-governance`) each carried a permanent warning.

  `validate_kind` now asks one more question before reporting: a `kind` outside
  `kinds` is **known** when it equals `derive_kind(role, stack)` with both axes
  present and both canonical. Each precondition is load-bearing, and the four
  ways of *not* being self-consistent all still report at WARNING — a
  hand-authored value that disagrees with its own axes, a one-off value in
  neither form, absent/half-declared/non-canonical axes, and an absent or
  non-string `kind`. `KIND_UNKNOWN` is not repurposed, renamed, or silenced,
  and the level stays WARNING. The axis lookup happens *after* the membership
  test, so a canonical `kind` still reads no axis set at all.

  `kinds` stays 12 and no declaration is written. Extending the list was the
  smaller diff and is ruled out in `design.md` §2: it would need re-extending
  for every future axis combination (the unbounded manual-maintenance point
  already found in `driftwatchdog`'s `GOVERNANCE_QUALITY_DECLARED_CODES`
  allow-list), it would freeze a derived field into a second source of truth,
  and it would break `test_existing_arrays_are_unchanged`, so adopting it means
  weakening a test to make a code change go green. Rewriting the 17
  declarations back to bare role words is rejected for the opposite reason: it
  reintroduces the role/stack conflation the split removed and would trade 17
  `KIND_UNKNOWN` for 17 `TAXONOMY_LEGACY_VALUE`.

  Nine tests pin both sides of the rule. **Verified to bite**: against the
  pre-fix body the two new-behaviour tests fail and the seven
  unchanged-behaviour tests pass, so the fix widens knowledge only where it
  should. The sweep test measures the whole 36-value space and asserts the
  unlisted count is non-zero, so it cannot pass on a vocabulary that already
  covered it.

  Measured: `python3 -m pytest tests` — **501 tests, OK** (492 prior + 9 new);
  fleet audit **errors=205 unchanged, warnings 35 → 18**, `KIND_UNKNOWN`
  **17 → 0**; `workspace_check --self-check` **0 errors / 0 warnings**;
  `openspec validate derived-kind-canonical-form --strict` passes;
  `openspec validate --changes --strict` 6/6; change names valid; `git diff
  --check` clean. The 18 remaining warnings are `HANDOFF_SCAFFOLD` 8,
  `DEPLOYMENT_PROMOTION_STALE` 7, `CAPABILITY_BLOCKED` 2,
  `DISCOVERABILITY_INVALID` 2, `CI_MISSING` 2,
  `QUALITY_README_CLAIM_UNSCOPED` 1 — none is a `KIND_UNKNOWN`, and no finding
  other than the 17 cleared ones moved. Spec delta MODIFIES the existing
  `Canonical kinds are enforced` requirement in
  `openspec/specs/portfolio-vocabulary/spec.md`; `identity-and-taxonomy` is not
  a target because that capability spec is still unarchived and its derived-kind
  requirement is unchanged by this fix. **Not archived, by instruction**, and
  `identity-and-taxonomy-axis-split` is likewise left unarchived.

  Evidence boundary: this change proves the checker no longer rejects a
  self-consistent derived value, and that it still rejects everything else. It
  does not prove that a repository's `kind` *means* anything — the axes remain
  descriptive and drive no required file, CI requirement, verification command,
  or evidence, deployment, or promotion state.

- `readme-claim-verification` (`f8df5ea`) evidence. A README status block is
  now checked against the tree. `QUALITY_README_CLAIM_CONTRADICTED` is ERROR
  and `QUALITY_README_CLAIM_UNSCOPED` is one aggregate coverage finding per
  run. `python3 -m unittest discover -s tests` — 489 tests, 0 failures (456
  prior + 33 new); `workspace_check --self-check` 0/0; `auto_gate --self-check`
  PASS; `check_skill_refs` ok; `openspec validate --all --strict` 18/18;
  `rules_drift` unchanged at 148; `jenkins_manifest --check` projects=0
  errors=0; `git diff --check` clean. At that commit the fleet audit moved
  `errors=211 warnings=17` → `errors=215 warnings=18`: +4 `CONTRADICTED`
  (argoset:31, care-bridge:17, retake-kit:42, devloom:9) and +1 aggregate
  `UNSCOPED` naming 3 statements. No other code changed count. **That +4 was
  itself a defect and is corrected at `833ca9c` below** — read the two numbers
  together, not the first alone.
  Read-only proved by hashing 17 real trees before and after a claim run.
  **The design's 7-repository corpus is stale and the change says so**: 6 of
  the 7 (`rostera`, `aesthra`, `reelio`, `pluggify`, `testify`,
  `shopifireman`, `care-bridge` were repaired 2026-10-03) now carry accurate
  claims.
  **The fleet is NOT red — the checker was wrong, and `833ca9c` fixed it.** The
  four `CONTRADICTED` findings recorded above were false positives, and the
  repair is in the check rather than in any README. The `deployment` evidence
  rule read the tree before the declaration, so a local `compose*.y(a)ml`
  counted as deployment evidence although all four repositories declare
  `deployable: false` (`devloom` holds a Compose *and* a `Dockerfile`, both
  local-only). The `application-source` synonyms carried bare `source`, so
  argoset's "live source adapters for named platforms" — provider language —
  resolved to the application-source surface. Both rules are narrowed, and
  neither fix blunts the check: rostera's original wording still yields its
  `build` and `ci` contradictions, and a repository declaring
  `deployable: true` while holding a Compose file is still a contradiction;
  both directions are pinned by regression tests.
  **Fleet audit is now `errors=211 warnings=18`** — the `errors=211`
  pre-change baseline exactly, with the four false-positive ERRORs gone. The
  added warning is the aggregate `QUALITY_README_CLAIM_UNSCOPED` naming 3
  statements that name no resolvable surface; it is a coverage metric, not an
  accusation, and it is meant to persist. Those four repositories need no
  README repair, and the repair queue in `tasks.md` §5.1 is withdrawn: editing
  an accurate document into a false one to satisfy a wrong rule is the
  failure this check exists to catch. Re-verified at `833ca9c`: 492 tests,
  0 failures (489 prior + 3 new regression tests), self-check 0/0, auto-gate
  PASS, drift unchanged at 148, `openspec validate --all --strict` 18/18.
  Nothing here gates on the check yet — binding it as a driftwatchdog gate
  check is deferred to that repository.

- `identity-and-taxonomy-axis-split` evidence. `role` and `stack`
  are now independent axes; `kind` is the derived, deprecated composite
  `<stack>-<role>`, or the bare `<role>` when stack is `none`. Three new codes:
  `TAXONOMY_MISMATCH` (ERROR), `TAXONOMY_LEGACY_VALUE`, `TAXONOMY_AXIS_UNKNOWN`
  (WARNING). **The migration has now been applied and the fleet committed**:
  81/81 declarations and 81/81 `projects.json` entries carry both axes;
  `argoscope`/`chronicleaf`/`devloom`/`kairovia` are registered as
  `product`/`dotnet` and `jenkins-bootstrap` is gone. From `/home/paul/code`
  the fleet audit moved **211 → 206 errors** (`errors=206 warnings=35`), with
  **zero** findings in any `TAXONOMY_*` family and `RULES_DRIFT_NEW` 0.
  **The check now enforces.** `TAXONOMY_AUDIT_ENABLED = True`; the flag is a
  committed constant, not an environment variable, and both of its states stay
  tested — held off, the same mismatched fixture is silent; on, it reports ERROR
  `TAXONOMY_MISMATCH`. `python3 scripts/axis_migration.py --check` exits 0.
  All 81 declarations satisfy `kind == derive_kind(role, stack)`, which is why
  `TAXONOMY_LEGACY_VALUE` is 0; the 13 `RULES_DRIFT_NEW` errors the four new
  registrations surfaced were cleared by adding the missing canonical
  `.ai-rules` paragraphs to those repositories, **not** by re-cutting the
  rules-drift baseline.
  `python3 -m unittest discover -s tests` — **492 tests, OK**;
  `openspec validate identity-and-taxonomy-axis-split --strict` passes;
  `git diff --check` clean. Three rulings were added during implementation, all
  in `design.md` §10: the migration also writes the derived `kind`, the drift
  guard consequently accepts the derived form so the tool stays idempotent, and
  `platform-rust` was a spec typo corrected to `rust-platform`. Two of those
  were owner rulings against the implementation as first written.
  Two things were **not** clean, both recorded in `tasks.md` §5 notes 7 and 8
  rather than fixed in passing: `workspace_check --self-check` was 1 error /
  1 warning (`devloom`'s non-canonical `lifecycle: scaffold`, now visible
  because the repository is registered; and this repository's own derived
  `python-governance` `kind`, which is absent from the deliberately
  byte-identical `vocabulary.json` `kinds` array), and
  `rollback-manifest.json` maps 77 of the 80 migrated siblings —
  `aegify`, `argoscope` and `argoset` have no before/after SHA pair, so a
  manifest-driven rollback would skip them.
  **Both of the self-check's two items are now clear at `97f6c68`**
  (`derived-kind-canonical-form`): `devloom` is registered `active`, and the
  self-check reads **0 errors / 0 warnings**. The `python-governance` warning
  was a checker defect — a correct derived value held to the frozen 12-value
  `kinds` list — not a declaration defect, so no portfolio file was written.
  `rollback-manifest.json` is extended to all 81 repositories at `4b6f49c` and
  is no longer outstanding.

- `rules-drift-gate-wiring` (`4a9edfc`) evidence. Archived as
  `2026-10-03-rules-drift-gate-wiring`; canonical spec
  `openspec/specs/rules-baseline/spec.md` (+4 added, 2 modified, 0 removed).
  The clause that `work-on-main-baseline-rule` added is now **enforced**, not
  merely reported. `rules-drift-baseline.json` holds the 140 clause-level gaps
  across 72 repositories; `workspace_check` emits ERROR `RULES_DRIFT_NEW` for
  any gap whose `(project, file, label)` is absent from it, and WARNING
  `RULES_DRIFT_BASELINE_STALE` when an entry is no longer reported. Of the
  148 total drift findings, 140 are clause-level and 8 are
  `HANDOFF_SCAFFOLD`, which is a different path and is not ratcheted.
  Measured: fleet audit **unchanged** at `errors=211 warnings=17` with zero
  `RULES_DRIFT_*` findings, and `rules_drift --check` still reports 148.
  `python3 -m unittest discover -s tests` — 389 tests, 0 failures (380 prior +
  9 new); `openspec validate --all --strict` 18/18 post-archive;
  `workspace_check --self-check` 0/0; `auto_gate --self-check --no-deployment`
  PASS; `check_skill_refs` ok; change names valid; `git diff --check` clean.
  Two implementation bugs were caught by the tests and are recorded in the
  commit: the stale-baseline comparison was reading a single-repository
  `--self-check` as if it were a fleet scan, and a fixture's stub rule files
  (`"w"` and `"c"`) read as six genuine clause gaps. Both fixed, and the
  fixture now loads the real shipped rule files so it cannot rot.
  The evidence boundary: this change proves the baseline is loaded fail-closed,
  that baselined debt does not block, and that unbaselined drift does. It
  cannot prove a repository's rule files are *read* by an agent, only that
  they are present and machine-checkable.

- `work-on-main-baseline-rule` (`bb4c406`) evidence:
  `python3 -m unittest discover -s tests` — 380 tests, 0 failures
  (376 prior + 4 new in `tests/test_rules_drift.py`: the clause reported
  when absent, absent when present, the opt-out marker suppressing it with
  the file's bytes unchanged, and both templates carrying the rule *and* its
  user-asks exception; plus 1 new test in `tests/test_decisions.py` keeping
  the 2026-09-30 date guarantee for the four seeded entries);
  `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings;
  `python3 scripts/auto_gate.py --self-check --no-deployment` — PASS;
  `python3 scripts/check_skill_refs.py` — `SKILL.md + 7 references` valid;
  `node scripts/check-openspec-change-names.mjs` — valid;
  `openspec validate --all --strict --no-interactive` — 18 passed, 0 failed
  post-archive (the change is archived and its delta merged into
  `openspec/specs/rules-baseline/spec.md`);
  `python3 scripts/rules_drift.py --root .. --check` — `drift_findings=148`,
  68 of them `work-on-main rule`, none for `workspace-governance`;
  `git diff --check` — clean before the handoff-only update. The canonical
  spec diff is purely additive: the `MUST NOT modify any project file`
  guarantee and the `<!-- workspace-governance:rules-customized -->` opt-out
  sentence are both intact. A 240-file sha256 snapshot across the whole
  fleet proves only this repository's own `AGENTS.md` and
  `.ai-rules/workflow.md` changed and **zero sibling files were touched**;
  `scripts/rules_drift.py` is a clause-only diff with no new environment
  variable, exemption registry or skip path. The first `openspec archive`
  run **aborted without writing**, because the delta was filed under a
  capability that did not exist; re-filing it under `rules-baseline` is why
  the archive reports `+4 added, ~2 modified, -0`.

- `python3 -m unittest discover -s tests -v` — 253 tests, 0 failures
  (217 prior + 36 new in `tests/test_standalone_project_boundary.py`:
  the six canonical statuses, the absolute-path and `..` escape
  rejections, optional and required integration branches, secret
  redaction, the bounded static scan, the adapter's new fields, the
  read-only guarantee, and the vocabulary loader).
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings
  (the governance repo carries no `standalone` block, so the
  audit is unchanged for it).
- `python3 scripts/auto_gate.py --self-check --no-deployment` — PASS.
- `python3 scripts/jenkins_manifest.py --check` — 0 projects, 0 errors
  (the standalone classification does not promote `deployable`).
- `node scripts/check-openspec-change-names.mjs` — valid.
- `openspec validate --all --strict --no-interactive` — 14 passed,
  0 failed (13 canonical specs, 1 active change; the
  `standalone-project-boundary` change is archived and now lives
  under `openspec/specs/`).
- `git diff --check` — clean before the handoff-only update.
- Portfolio audit (`--root .. --audit --json`) is unchanged in shape:
  211 errors and 17 warnings, with no new project-level finding
  codes introduced by this change. The `STANDALONE_*` codes only
  fire on declarations that opt in, and no adopted project in this
  workspace has opted in.
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings.
- `python3 scripts/workspace_check.py --decisions` — prints D1–D4 summary;
  `--json` produces a parseable document with `{id,title,status,date}` per
  entry.
- `python3 scripts/workspace_check.py --root .. --audit --json` — same 211
  errors / 17 warnings shape as before; no new project-level finding codes
  introduced by this change. The +4 errors / -1 warning vs the prior
  HANDOFF baseline is pre-existing sibling-repo drift, not from this
  change.
- `python3 scripts/runtime_templates.py list` — ten version-one template
  families (six container-backed, four external) listed from the registry.
- `node scripts/check-openspec-change-names.mjs` — valid.
- `openspec validate --all --strict --no-interactive` — 14 passed, 0 failed
  (12 canonical specs, 1 active change, `external-postgres-runtime-profile`
  archived and now lives under `openspec/specs/`).
- `python3 scripts/auto_gate.py --self-check --no-deployment` — PASS.
- `git diff --check` — clean before the handoff-only update.

Prior evidence (package 5, `post-mvp-readiness`):

- `python3 -m unittest discover -s tests` — 167 tests, 0 failures.
- `python3 scripts/check_skill_refs.py` — `SKILL.md + 7 references` valid.
- `gh auth status` — authenticated account `lileililiwen`.
- `gh repo create` created `https://github.com/lileililiwen/workspace-governance`,
  `main` was pushed through `origin`, and
  `python3 scripts/github_metadata.py . --check --json` reports
  `metadata_verified` with no differences. The local declaration records the
  verified GitHub publication state.
- Portfolio audit `python3 scripts/workspace_check.py --root .. --audit
  --json` — 207 errors and 18 warnings, intentionally exposing migration work:
  57 `LICENSE_MISSING`, 75 `DISCOVERABILITY_MISSING`, and 75
  `PUBLICATION_UNVERIFIED`, plus the pre-existing handoff/deployment and
  capability warnings.
- `python3 scripts/jenkins_manifest.py --check --root ..` — blocked by the
  same newly exposed portfolio metadata gaps; no deployment manifest changed.
- `python3 scripts/runtime_templates.py list` — six version-one template
  families listed from the registry.
- Runtime template tests — all six families rendered; generated Compose files
  passed `docker compose config` in the available Docker environment.
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings
  after adding deployment-template metadata validation.
- `openspec validate --changes --strict --no-interactive` — 1 passed, 0 failed
  before archive; the archived and canonical specs are now present.

Six changes shipped in sequence.

**1. `registry-and-declaration-integrity`** (archived
`openspec/changes/archive/2026-09-26-registry-and-declaration-integrity/`,
canonical spec `openspec/specs/registry-integrity/spec.md`). Registered the 11
discovered declarations, corrected six declaration identity/format defects,
created four missing declarations, added duplicate id/path guards, derived and
enforced the canonical project id in `init_project.py`, and made
`maturity_audit.py` report the authoritative `workspace_check` counts.
Unregistered discovery is now an ERROR via
`discovery.unregistered_policy: "fail"`.

**2. `shared-rules-single-source`** (archived
`openspec/changes/archive/2026-09-26-shared-rules-single-source/`, canonical
spec `openspec/specs/rules-baseline/spec.md`). Made `templates/project/**` the
only source of the shared rules and had `init_project.py` read it from disk,
deleting the duplicate embedded copy that had drifted from the on-disk
templates. Added `scripts/rules_drift.py`, a **non-destructive** clause-level
drift report with no `--apply`, and surfaced scaffold `HANDOFF.md` files from
the audit as `HANDOFF_SCAFFOLD`. Created `crossalheart`'s missing rules,
clearing the workspace audit's last two errors.

**3. `portfolio-vocabulary-normalization`** (archived
`openspec/changes/archive/2026-09-27-portfolio-vocabulary-normalization/`,
canonical spec `openspec/specs/portfolio-vocabulary/spec.md`). Added
`vocabulary.json` as the single machine-readable source for the 15 canonical
profiles, 12 canonical kinds, placeholder markers, and secret-field
substrings. `workspace_check.py` enforces `PROFILE_UNKNOWN` (ERROR),
`KIND_UNKNOWN` (WARNING), and `PROFILE_DOC_UNKNOWN` (ERROR) from it and exits
2 with `VOCABULARY` when it is missing or unusable; `product_code_quality.py`
defaults its markers from it while keeping per-project overrides;
`maturity_audit.py` reports the vocabulary version. Added the 9 missing
`profiles/*.json` documents and remapped 19 sibling declarations with
non-canonical `kind`/`profile` values (left uncommitted in their own
repositories, alongside the earlier declaration/rules edits).

**4. `governance-front-door`** (archived
`openspec/changes/archive/2026-09-27-governance-front-door/`, canonical spec
`openspec/specs/governance-front-door/spec.md`). Shipped
`skills/workspace-governance/SKILL.md` — one auto-triggering entry whose
routing table sends each governance situation to a per-concern reference —
plus six references that link owning scripts and canonical specs instead of
restating policy. Added `scripts/check_skill_refs.py`, which fails naming the
reference and the missing path when a cited path or command does not exist,
with `tests/test_skill_refs.py` covering the valid tree, the missing-script
negative, routing resolution, the safe default, and the
no-destructive-instructions boundary. Documented the manual install in
`README.md`. No checker, policy, spec, template, or sibling repository was
changed.

**5. `post-mvp-readiness`** (archived
`openspec/changes/archive/2026-09-27-post-mvp-readiness/`, canonical spec
`openspec/specs/readiness/spec.md`). Docs-only readiness for the shared
governance owner: a consolidated README Usage section with one copy-paste
example per command (every flag verified against the shipped `--help`;
the Forge adapter documents its stdin/stdout contract), a redacted
checked-in capture `docs/assets/terminal-self-check.txt` showing a real
`--self-check` (0 errors, 0 warnings) and `--audit --json` (0 errors,
16 warnings with the 9/7 breakdown) run plus one labelled illustrative
finding, an explicit screenshots-not-applicable note (CLI-only), and a
single vocabulary entry point (`docs/vocabulary.md` plus a README
section) that links the canonical sources and states the
intent-vs-evidence invariant. Also repaired the stale
`docs/package-backlog.md` dependency order (now records 1, 2, 3, 8
shipped). No script, checker, schema, template, registry, promotion,
or spec-behavior change; the pre-existing dirty `.project.json` and
untracked `MATURITY_PROGRESS_SUMMARY.md` were deliberately left
unstaged.

The read-only **Forge governance adapter**
(`scripts/forge_governance_adapter.py`) is unchanged; its canonical spec is
`openspec/specs/forge-governance-adapter/spec.md`.

**7. `shared-postgres-runtime-profile`** (archived
`openspec/changes/archive/2026-10-01-shared-postgres-runtime-profile/`,
canonical spec `openspec/specs/external-postgres-runtime-profile/spec.md`).
Adds four external-PostgreSQL runtime template families
(`dotnet-web-postgres-external`, `rust-web-postgres-external`,
`python-web-postgres-external`, `worker-postgres-external`) that connect an
app container to a supplied PostgreSQL service without starting a database
container, image, volume, or provisioning command. `runtime_templates.py`
gains `optional_variables` support (e.g. `db_port` defaulting to `5432`),
refuses a `!.env*` unignore rule with `SECRET_IGNORE_CONFLICT` before any
write, idempotently amends the root `.gitignore` to ignore `.env`, and
records `database_source: external` in the project declaration. The
generated `.env.example` has a blank `PGPASSWORD`; the compose file reads
`PGHOST`/`PGPORT`/`PGUSER`/`PGDATABASE`/`PGPASSWORD` from the runtime
environment and never substitutes a password. The deployment schema gains
`database_source` (`container` | `external`); the existing six per-project
container-backed templates are byte-identical. The `runtime-template-renderer`
spec was extended with external-profile scenarios; the canonical
`external-postgres-runtime-profile` spec captures the new contract. No
sibling repository, profile, capability schema, promotion-ledger, Jenkins
consumer, or DriftWatchdog implementation was changed.

**6. `toolchain-decision-register`** (archived
`openspec/changes/archive/2026-10-01-toolchain-decision-register/`,
canonical spec `openspec/specs/workspace-decision-register/spec.md`).
Adds `decisions.json` at the governance root, a fail-closed
`scripts/decisions.py` loader mirroring the vocabulary discipline, and a
`workspace_check.py --decisions` summary path. `workspace_check.py`
self-check reports `DECISIONS_REGISTER_MISSING` as ERROR when the file
is absent; portfolio audits emit a single `DECISIONS_REGISTER_ABSENT`
WARNING; broken shapes and unknown registry references are always
errors. Seeded with the four accepted toolchain decisions resolved on
2026-09-30 (D1 agent-runtime split, D2 self-hosted Zitadel with the
public-catalog exclusion restated in `policy_boundary`, D3
capability-registry home split with schema=platform-contracts and
instances=workspace-governance, D4 crates.io publication). The D2
exclusion and the D3 split are asserted by string in
`tests/test_decisions.py`. `docs/toolchain-decisions.md` is the human
view; `docs/capabilities.md` records the D3 ownership note. No
sibling repository, runtime template, profile, capability schema, or
promotion-ledger was changed; the public-site boundary in D2 is
restated from the root ownership table and not weakened.

**7. `standalone-project-boundary`** (archived
`openspec/changes/archive/2026-10-01-standalone-project-boundary/`,
canonical spec `openspec/specs/standalone-project-boundary/spec.md`).
Adds a fail-closed standalone-readiness contract: a project may declare
a `standalone` block and an `integrations` array in `.project.json`,
and the audit classifies it as one of six canonical statuses —
`standalone`, `standalone-with-optional-integrations`,
`library-dependent`, `workspace-dependent`, `service-dependent`,
`blocked` — without executing any project command. The schema adds
optional `standalone` and `integrations` blocks plus `$defs.standalone_status`
and `$defs.integration`; existing declarations remain valid (absent
block ⇒ `blocked` + `STANDALONE_BLOCKED` warning, never a pass).
`vocabulary.json` gains `standalone_statuses` (6 values) and
`integration_required_states` (2 values), and the loader fails closed
on missing, empty, non-array, or duplicate values. `workspace_check.py`
adds `validate_standalone` and `classify_standalone` plus 12 finding
codes (`STANDALONE_BLOCKED`, `STANDALONE_STATUS_INVALID`,
`STANDALONE_SHAPE`, `STANDALONE_ABSOLUTE_PATH`,
`STANDALONE_PATH_ESCAPE`, `STANDALONE_REQUIRED_SIBLING`,
`STANDALONE_REQUIRED_SERVICE`, `STANDALONE_OPTIONAL`,
`STANDALONE_INTEGRATION_PENDING`, `STANDALONE_EVIDENCE_MISSING`,
`STANDALONE_SECRET_VALUE`, `STANDALONE_DETECTED_SIBLING`). The
bounded static scan reads only `Cargo.toml`, `package.json`,
`pyproject.toml`, and `go.mod`; it never executes the project and
never overrides the declared status. The Forge governance adapter
exposes the same classification as four additive observation fields
(`standalone_status`, `standalone_observed_revision`,
`standalone_integrations`, `standalone_findings`); existing keys,
status mappings, and exit codes are unchanged, and
`build_standalone_observation` never raises. No sibling repository,
runtime template, profile, capability schema, promotion-ledger,
Jenkins consumer, or DriftWatchdog implementation was changed.

## Authored, not yet implemented

Four packages are fully authored as OpenSpec changes and are ready to implement
in a fresh conversation. Each lives in the repository that owns the work:

- `driftwatchdog/openspec/changes/gate-evidence-export/` (package 4).
- `dotnet-platform-libs/openspec/changes/platform-consumer-bootstrap/` (package 5).
- `rust-platform-libs/openspec/changes/rust-platform-consumer-baseline/` (package 6).
- `platform-contracts/openspec/changes/contracts-consumer-parity/` (package 7).

`docs/package-backlog.md` is the index and the coordination rules; the detail
lives in each change directory, so a package has exactly one source of truth.

## Evidence

- Package 12 (`standalone-project-boundary`) evidence:
  `python3 -m unittest discover -s tests` — 253 tests, 0 failures
  (217 prior + 36 new in `tests/test_standalone_project_boundary.py`:
  the six canonical statuses, the absolute-path and `..` escape
  rejections, optional/required integration branches, secret
  redaction, the bounded static scan, the adapter's new fields, the
  read-only guarantee, and the vocabulary loader);
  `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings
  (the governance repo carries no `standalone` block, so the audit
  is unchanged for it);
  `python3 scripts/auto_gate.py --self-check --no-deployment` — PASS;
  `python3 scripts/jenkins_manifest.py --check` — 0 projects, 0 errors
  (the standalone classification does not promote `deployable`);
  `node scripts/check-openspec-change-names.mjs` — valid;
  `openspec validate --all --strict --no-interactive` — 14 passed,
  0 failed (13 canonical specs, 1 active change;
  `standalone-project-boundary` archived and now lives under
  `openspec/specs/standalone-project-boundary/spec.md`);
  `git diff --check` — clean before the handoff-only update.
  The portfolio audit (`--root .. --audit --json`) is unchanged in
  shape: 211 errors and 17 warnings, with no new project-level
  finding codes introduced by this change (the `STANDALONE_*` codes
  only fire on declarations that opt in, and no adopted project in
  this workspace has opted in). The vocabulary loader now
  fail-closes on missing, empty, non-array, or duplicate
  `standalone_statuses` and `integration_required_states` (asserted
  by `tests/test_standalone_project_boundary.py::VocabularyStandaloneTests`).
  The Forge governance adapter always emits the four new fields
  alongside the existing keys; existing keys, status mappings, and
  exit codes are unchanged (asserted by
  `tests/test_forge_governance_adapter.py`); secret-shaped values
  in `standalone` and `integrations` fields are replaced by
  `[redacted]` in evidence lines, and the read-only guarantee is
  preserved (asserted by
  `tests/test_standalone_project_boundary.py::AdapterStandaloneObservationTests`).
  No sibling repository, runtime template, profile, capability
  schema, promotion-ledger, Jenkins consumer, or DriftWatchdog
  implementation was changed.

- Package 11 (`shared-postgres-runtime-profile`) evidence:
  `python3 -m unittest discover -s tests` — 217 tests, 0 failures
  (204 prior + 13 new in `tests/test_runtime_templates.py`: four
  `*-postgres-external` template metadata and rendering tests, an
  idempotence test for `.gitignore`, a `SECRET_IGNORE_CONFLICT` test
  for the unignore rule, a `TARGET_EXISTS`-before-`.gitignore` test
  for existing `compose.yaml`, a `db_port` default test, a
  secret-shaped-variable rejection test, and a container/internal
  template regression test);
  `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings;
  `python3 scripts/auto_gate.py --self-check --no-deployment` — PASS;
  `python3 scripts/jenkins_manifest.py --check` — 0 projects, 0 errors;
  `python3 scripts/runtime_templates.py list` — ten version-one
  template families (six container-backed, four external);
  `node scripts/check-openspec-change-names.mjs` — valid;
  `openspec validate --all --strict --no-interactive` — 14 passed, 0 failed
  (12 canonical specs, 1 active change; `shared-postgres-runtime-profile`
  archived and now lives under
  `openspec/specs/external-postgres-runtime-profile/spec.md`);
  `git diff --check` — clean before the handoff-only update. The portfolio
  audit (`--root .. --audit --json`) is unchanged in shape: 211 errors and
  17 warnings, with no new project-level finding codes introduced by this
  change (the +4 errors / -1 warning vs the prior HANDOFF baseline is
  pre-existing sibling-repo drift, not from this change). All four new
  external templates rendered end-to-end and passed `docker compose config`
  with a dummy `.env` (verified by `check_target`); the existing six
  per-project container-backed templates remained byte-identical to their
  pre-change bytes (verified by `git diff` against `HEAD~`); no password
  or secret-shaped value was ever supplied as a renderer argument, written
  to `.env.example`, or stored in `.project.json`; the renderer refuses
  before any file is written when the root `.gitignore` contains a
  `!.env*` rule or an existing `compose.yaml`/`Dockerfile` is not listed
  via `--replace`.

- Package 10 (`toolchain-decision-register`) evidence:
  `python3 -m unittest discover -s tests` — 204 tests, 0 failures
  (167 prior + 37 new in `tests/test_decisions.py`);
  `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings;
  `python3 scripts/workspace_check.py --decisions` — prints D1–D4; `--json`
  produces a parseable document with `{id,title,status,date}` per entry;
  `python3 scripts/auto_gate.py --self-check --no-deployment` — PASS;
  `node scripts/check-openspec-change-names.mjs` — valid;
  `openspec validate --all --strict --no-interactive` — 14 passed, 0 failed
  (10 canonical specs, 4 active changes; `workspace-decision-register`
  archived to `openspec/specs/workspace-decision-register/spec.md`);
  `git diff --check` — clean before the handoff-only update. The portfolio
  audit (`--root .. --audit --json`) is unchanged in shape: 211 errors and
  17 warnings, with no new project-level finding codes introduced by this
  change (the +4 errors / -1 warning vs the prior HANDOFF baseline is
  pre-existing sibling-repo drift, not from this change). The D2 public-
  catalog exclusion and the D3 capability-registry split are asserted by
  string in `tests/test_decisions.py`; the seeded downstream references
  all resolve to live registry ids.

- Package 9 (`post-mvp-readiness`) evidence:
  `python3 -m unittest discover -s tests` — 148 tests, 0 failures;
  `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings;
  `python3 scripts/workspace_check.py --root .. --audit --json` — 0 errors,
  16 warnings (9 `HANDOFF_SCAFFOLD`, 7 `DEPLOYMENT_PROMOTION_STALE`);
  `python3 scripts/jenkins_manifest.py --check --root ..` — 7 projects,
  0 errors; `python3 scripts/auto_gate.py --self-check --no-deployment` —
  PASS; `node scripts/check-openspec-change-names.mjs` — valid;
  `openspec validate --all --strict --no-interactive` — 9 passed, 0 failed
  (9 canonical specs, 0 active changes); `git diff --check` — clean.
  Every documented Usage command runs `--help` (adapter: stdin contract);
  every new README/docs relative link resolves on disk; no external image
  host; capture redacted and reproducible offline.
- Package 8 evidence still stands:
  `python3 scripts/check_skill_refs.py` — `skill refs ok: SKILL.md + 6 references`.
- `python3 -m unittest discover -s tests` — 148 tests, 0 failures (140 prior + 8 new in `tests/test_skill_refs.py`).
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings (unchanged).
- `python3 scripts/workspace_check.py --root .. --audit --json` — **0 errors**,
  16 warnings (9 `HANDOFF_SCAFFOLD`, 7 `DEPLOYMENT_PROMOTION_STALE`; unchanged
  by this docs-only change).
- `node scripts/check-openspec-change-names.mjs` — valid;
  `openspec validate --all --strict --no-interactive` — 9 passed, 0 failed
  (8 canonical specs, 1 active change).
- `git diff --check` — clean.
- Every cited `python3 scripts/*.py` command runs `--help` successfully
  (workspace_check, init_project, auto_gate, product_code_quality,
  maturity_audit, jenkins_manifest, rules_drift, check_skill_refs).
- Package 3 evidence still stands (140-test baseline, 0/0 self-check,
  0-error portfolio audit, vocabulary version 1, quality PASS, Jenkins
  manifest 7 projects 0 errors, auto-gate PASS, adapter observations).
- Commit `7e1bf63` was the pre-work HEAD; package 8 implementation is the
  first commit after it, followed by this HANDOFF-only commit.

## Portfolio impact observed

The `.ai-rules` baseline change is deliberately non-destructive. A first
implementation made whole canonical paragraphs mandatory and shipped `--apply`;
running it appended a duplicate canonical block to **60 project files**,
including well-maintained ones such as `mewo` whose text already expressed the
same rules. That was measured, judged a regression, and reverted byte-for-byte
before the change was re-scoped to a report-only clause check. The portfolio
`git diff` for `.ai-rules` is clean apart from `crossalheart`'s newly created
files.

9 adopted projects still carry scaffold `HANDOFF.md` bodies (reported as
`HANDOFF_SCAFFOLD`; two products recorded real handoffs since 2026-09-26).
The 7 `DEPLOYMENT_PROMOTION_STALE` warnings remain triaged:
re-promotion was not performed because it would record a deployable decision at
the current revision without a new release.

Sibling repositories still hold uncommitted declaration, rules, and vocabulary
remap files created by the first three changes (the 10 declarations plus
`crossalheart`'s rules and `.ai-gate`, plus 19 `kind`/`profile` remaps);
they were left uncommitted in their own repositories. Package 8 touched no
sibling repository.

## Next action

1. **`bind-declared-gate-checks` is implemented, tested, validated, and
   partly closed; still deliberately not archived.** The fleet write landed
   and `gate-debt.json` was regenerated against it.

   | Measurement | Before | After |
   |---|---|---|
   | `governance-quality` UNBOUND across the fleet | 41 | **0** |
   | Debt record, unresolved checks | 168 | **127** |
   | Debt record, repositories carrying debt | 59 | **54** |
   | Debt record vs the gate's own unbound set | equal | **equal (127 = 127)** |
   | `python3 -m pytest tests -q` | 725 passed | **734 passed**, 158 subtests, 0 failed |

   The 127 that remain are 80 `placeholder-threshold` / `product-code-boundary`
   pairs (40 each) plus 47 shape refusals. Every one names the file and key
   whose absence caused it — for the product-quality pair, the `quality` block
   missing from that repository's own `.project.json`, and the checker's exit-2
   behaviour without it.

   **The `checks:` boundary moved, and this is the design call to review.** The
   binder used to refuse to touch `checks:` at all, so a repository naming
   `profile: product` was failed by the gate for three unbound checks it could
   never close. That guarded the wrong boundary: the gate enforces one rule —
   `commands:` must not name a check the manifest does not select
   (`driftwatchdog/src/gate/aigate.rs:213`) — not "`checks:` is never written".
   The binder now writes both halves in one operation, and only when a checker
   is reachable; where none is, neither half is written, because a selection
   with no command is a worse state than the gap. The spec amendment says so
   plainly and names the test that guarded the wrong boundary.

   **Both command flags are load-bearing, and dropping either is a silent
   wrong answer.** Without `--root ..` the checker defaults to this
   repository and reports 81 false `PROJECT_MISSING` findings. Without
   `--project <repo>` it audits the whole workspace, and the gate's concern
   filters findings by **code, not project** — measured on this fleet, that
   charges `bargora` for 12 findings of which 0 are its own (4 from
   `chronicleaf`, 3 from `metiscope`, 3 from `openlearning`, 1 from
   `dotnet-platform-libs`). Scoped, the same repository reads PASS.

   **Re-measured on real gate runs, not dry runs.** The 8 repositories whose
   only failure was these bindings (`bargora`, `crossalheart`, `dharmatlas`,
   `hypora`, `mnemyx`, `mortalect`, `namevia`, `triplanify`) all read
   `governance-quality: PASS` after the binding, and each is blocked only by
   the two product-quality concerns that have no policy to run. All 76
   manifests were checked for the invariant that matters: **0** `commands:`
   entries name a check the manifest does not select.

   **Not done, on purpose.** No `quality` block was written anywhere, so the
   80 product-quality checks stay unbound — `product_roots`, `languages` and
   `placeholder_threshold` are per-repository owner decisions and 79 of 81
   repositories have not written one. `forge_governance_adapter.py` remains
   unbindable by construction (it reads stdin; the gate gives the child
   `Stdio::null()`) and is not wired. `README.md` is still not updated
   (3.7). Nothing was pushed; all 40 fleet commits are local on `main`.
2. Archive `derived-kind-canonical-form` and `identity-and-taxonomy-axis-split`
   when the owner is ready. Both are implemented, committed, and validated
   strictly; neither is archived. Archiving the first folds the MODIFIED
   `Canonical kinds are enforced` requirement into
   `openspec/specs/portfolio-vocabulary/spec.md`, and archiving the second is
   what finally lands the `identity-and-taxonomy` capability spec. Archiving
   them in that order keeps the derived-kind requirement on `main` from being
   split across two histories.
3. `spec-purpose-backfill-gate` (`6de0dd7`) is implemented, tested, and
   validated, and is deliberately **not archived**: the fence is shut and the
   gate-wiring dependency is unmet. Two things gate its archive, in order.
   First, `driftwatchdog`'s `GOVERNANCE_QUALITY_DECLARED_CODES` must gain
   `"SPEC_PURPOSE_PLACEHOLDER"`, or the concern reports `PASS` while this
   checker exits 1 — a cross-repo step this change does not own. Second, the
   1,234-file fleet write is a cross-repo source edit needing explicit
   authorization: review the dry run, then `--apply`, then set
   `SPEC_PURPOSE_AUDIT_ENABLED = True` in the same commit. Until both are done,
   do not enable the rule and do not archive the change.
4. Take packages 4–7 to their owning repositories via `docs/package-backlog.md`;
   each is a separate conversation and a separate repository.
5. Replace the 9 scaffold `HANDOFF.md` bodies as each product's own state is
   recorded (product-owned work).
6. Re-promote each pilot project at its current `HEAD` once its release pipeline
   produces real artifacts, and replace the pilot-context `release_evidence`
   placeholders.
7. Commit the sibling declaration/rules edits in their own repositories when
   those repositories' pending work is ready.

## Evidence boundary

This repository can prove registry/schema/checker behavior, capability
declaration shape, release-evidence shape, the deployable+release-evidence
gate, the product-code quality rule, the deployment promotion
mechanism, the rules-baseline drift report, and the Forge governance adapter
contract. It can prove the front-door skill routes to existing references
(`check_skill_refs.py`), but it cannot prove an agent will trigger the skill. It cannot prove that the seven promoted projects actually build, pass
tests, deploy, or are runtime-healthy; those claims require the promoted
projects' own verification commands and jenkins-local runtime observation. The
adapter likewise cannot prove a project's runtime or release health;
it only translates this repository's own audit findings.

## OpenSpec workflow

Select one active change with `openspec list`, run the local checker and
project gate before archive, validate strictly, archive canonical specs, then
create the required two commits. Each completed OpenSpec spec/change requires
exactly two commits: first the implementation/tests/archive commit, then a
HANDOFF.md-only pointer/evidence commit.

## The marker vocabulary is corrected and the fleet write is done

`placeholder-threshold` and `product-code-boundary` were 78 unbound checks in 39
repositories — the second-largest gap in the fleet. Both are answered by
`scripts/product_code_quality.py`, which is fail-closed and exits 2 without a
`quality` block in the repository's own `.project.json`.

`scripts/bind_gate_commands.py` derives the block (`derive_quality_block`) and
writes it with the command in one operation, both under `--apply` and both
behind the dirty check. `product_roots` and `languages` come from the same disk
detection the build commands use, with the manifest each root came from as
evidence. The `src` convention is per manifest kind: a `.csproj` at
`src/Foo/Foo.csproj` yields `src/Foo`, not `src/Foo/src`. `placeholder_threshold`
is a judgement, written as `0` — the deliberately strict default, relaxable per
project. No `exclude` and no marker list are invented.

**The vocabulary was the blocker, and it is corrected.** Over
`driftwatchdog/src/` the word `placeholder` matched **26** times and **zero** of
them was unfinished work: every one was prose describing the word
(`//! ... extends this placeholder`, `/// Ordered, comma-separated placeholder
names`) or the gate's own `placeholder-threshold` concern identifier in
`src/gate/adapters.rs`, against 1 `TODO` and 1 `FIXME` on the same tree.
`vocabulary.json` now holds `todo`, `fixme`, `xxx`, `hack`, `unimplemented!`,
`todo!`. `product_code_quality.py` is unchanged — it already resolved its
markers from that file at use time.

**This reduces the finding count, and the number is not hidden.** Across the 39
repositories written, marker findings fall **468 → 38** and repositories
reporting `FAIL` fall **28 → 13**. 430 of the 468 were the two ordinary English
words. It is a correction to a mis-specified vocabulary in the one file that
owns it: no threshold moved, no `exclude` was added, no test was skipped or
filtered, and every `TODO` and `FIXME` in the fleet is still reported. The spec
delta at
`openspec/changes/bind-declared-gate-checks/specs/portfolio-vocabulary/spec.md`
states this and keeps the correction-versus-filter distinction explicit.

A hand-written per-project `placeholder_markers` list is left exactly as its
author wrote it. Every `.project.json` in the fleet was grepped: exactly one
carries an inline list — `forge` — and it already names the six retained
tokens, so the earlier premise that it included `placeholder` and `stub` did not
hold, and there was no conflict to resolve.

### Fleet outcome

| Measurement | Before | After |
|---|---|---|
| Gate unbound checks | 125 | **47** |
| Gate repositories with an unbound check | 53 | **24** |
| `gate-debt.json` `unresolved_checks` | 125 | **47** |
| `gate-debt.json` `repositories_with_debt` | 53 | **24** |
| Set-equal against the gate | yes | yes |
| `placeholder-threshold` unbound | 39 | **0** |
| `product-code-boundary` unbound | 39 | **0** |
| Repositories fully wired (of 76 with a manifest) | 23 | **52** |

Set equality, never a count, against
`driftwatchdog/target/debug/driftwatchdog`. The `driftwatchdog` on `PATH` is a
2026-10-01 `cargo install` build that predates unbound reporting and answers 0
for the whole fleet; it is not a usable oracle.

Confirmed on real gate runs, not only on the checker. `openlearning`:
`placeholder-threshold FAIL — placeholder marker count 18 exceeds threshold 0`,
`unbound: ['tests']`. `pangoman`: `placeholder-threshold PASS — placeholder
marker count 0 at or below threshold 0`, `unbound: []`. Both checks execute and
return verdicts in both directions, which is the point of binding them.

**Regenerating the record can no longer write the fleet.** `--debt-only` threads
`apply=False` into `run()` and persists only `gate-debt.json`; it refuses to
combine with `--apply` (exit 2). `DebtOnlyTests` (4) pins that. Note the
`--apply` default `--debt-out` is `root/gate-debt.json`, so an apply run against
`--root /home/paul/code` writes a stray record at the workspace root — pass
`--debt-out` explicitly. The one produced here was removed with
`Path.unlink(missing_ok=True)`.

**Fleet scope, verified.** 39 repositories written, `.project.json` and
`.ai-gate/gate.yaml` only, one commit each on `main`, no push. Confirmed by
hashing both files across all 81 repositories before and after (78 files, 39
repositories, empty set difference against the authorised list) and by
`git show --name-only` on every one of the 39 commits. Pre-existing uncommitted
work in those repositories from earlier sessions — including
`openaccounting/Cargo.toml`, `rust-ecommerce/Cargo.toml` and the `*.bak-profile`
files — was left untouched.

Suite: **748 passed, 158 subtests, 0 failed** (741 on arrival). Seven tests
added: 3 in `PlaceholderTests`, 4 in `DebtOnlyTests`.
`test_placeholder_markers_unchanged` asserted the old list and is now
`test_placeholder_markers_are_unambiguous_debt_tokens`, pinning the retained set
and asserting `placeholder` and `stub` are absent. Negative control: putting
both words back raises marker findings 38 → 468 and red repositories 13 → 28;
the file was restored byte-identically and the suite returned to 748.

### The Rust boundary check reads the module tree now

The defect this section used to list as an owner's problem is fixed, in commit
`93e60cf`, and the fix is in the checker rather than in any repository. See
`openspec/changes/bind-declared-gate-checks/tasks.md` §8 and
`openspec/changes/bind-declared-gate-checks/specs/product-code-quality-boundary/spec.md`.

`product_code_quality.py` decided "is this Rust file test code" from the file
alone, so it could not see that `src/gate/governance/tests.rs` is only ever
compiled by `#[cfg(test)] mod tests;` in `src/gate/governance/mod.rs:627`. A
second, separate cause sat behind it: the guard was tracked per line rather than
per item, so an attribute written *between* the guard and its item disarmed it —
`#[cfg(test)]`, `#[path = "tests.rs"]`, `mod tests;` is one guarded item and was
read as none.

| `product-test-code` findings | Before | After |
|---|---|---|
| `driftwatchdog` | 24 | **0** |
| `rust-ecommerce` | 18 | **0** |
| `alltools-platform` | 2 | **1** |
| fleet total | 60 | **17** |

Real gate runs, `driftwatchdog/target/debug/driftwatchdog gate`:

| Repository | `product-code-boundary` | Still red for |
|---|---|---|
| `rust-ecommerce` | FAIL → **PASS** | `security` unbound, `source-file-size`, two adapter timeouts |
| `driftwatchdog` | FAIL (24 findings) → FAIL (0 test-code) | 2 placeholder markers in `src/gate/concerns.rs:20`, 8 files over 1000 lines |
| `alltools-platform` | FAIL | 1 real bare `#[test]` in `test_prompt.rs`, 11 markers |
| `argoset`, `crossify`, `fotofy`, `transitfy` | FAIL, unchanged | the **TypeScript** `describe(`/`it(` heuristic — a different defect |

The five repositories that were red on `product-test-code` alone with zero
markers — `argoset`, `crossify`, `fotofy`, `rust-ecommerce`, `transitfy` — split
one and four. Only `rust-ecommerce` was red for the Rust reason; the other four
are red on a TypeScript regex, which this change does not touch.

No `exclude`, no filename exception list, no test weakened. A `tests.rs` that
no `#[cfg(test)]` declaration reaches is still reported, which is what
`RustModuleTreeBoundaryTests::test_the_declaration_is_what_clears_it_not_the_file_name`
pins. Negative control on a **copy** of `src/gate/governance` in a temp
directory, never on `driftwatchdog` itself: **0 → 24 → 0** findings. A `tests/`
directory inside a product root needed no handling — `DEFAULT_EXCLUDED_DIRS`
already matches the `tests` path segment, so it is out of scope before the module
tree is consulted.

Suite: **759 passed, 158 subtests, 2 failed** (13 added here). Both failures
are independent of this change and reproduce with it stashed:

- `GateOracleTest::test_the_record_names_nothing_the_gate_calls_bound` —
  `gate-debt.json` calls `('metiscope', 'build')` debt the live gate does not.
  Pre-existing on `main`; it is about the debt record, not the Rust scan.
- `SequencingGateTests::test_the_gate_is_open_and_the_fleet_corpus_is_clean` —
  `refused: lileililiwen-site: … census: 12 .py records kind 'product' but the
  declaration says 'deployment'`. It reads the live sibling tree at
  `/home/paul/code`, and `lileililiwen.github.io` has gained `scripts/*.py`
  and `tests/*.py` from other work.

The `748 passed, 0 failed` figure quoted above was true when it was measured.

### Open for their owners

- **Four repositories are red on the TypeScript `describe(`/`it(` heuristic**
  with zero marker findings: `argoset` 2, `crossify` 1, `fotofy` 6,
  `transitfy` 6. The same class of false positive as the Rust one — a component
  whose own name or subject is "test" trips a two-token regex — in a language
  this change does not touch. Recorded in `tasks.md` §8; not fixed here.
- **`opendockify` is `REVIEW_REQUIRED` on `readme-claim-unscoped`** — its README
  line 52 makes a negative status statement naming no surface the checker can
  resolve. A README finding for that repository's owner, surfaced only now
  because the block made the check run.
- The change is **not archived**. Archiving needs separate authorization.

## A new change now waits on the gate debt

`QUALITY_NEW_CHANGE_WITH_GATE_DEBT` is a new `ERROR` in
`scripts/workspace_check.py`: a repository listed in `gate-debt.json` that opens
an OpenSpec change which is neither `close-unbound-gate-checks` nor already
recorded in `gate-debt-baseline.json`. It has no fence, because a precondition
is *supposed* to be red while debt is open, and it is fail-closed: all three
records — the debt, the ratchet, the exceptions — are refused rather than
defaulted when missing, corrupt or mistyped.

**It is a ratchet, not a latch, and that took one correction.** The first cut
keyed on "has any unarchived change" and failed all 22 debted repositories on
arrival, `workspace-governance` included on its own `--self-check`. That is a
deadlock: an open change can only be closed by finishing it, so the rule forbade
the only action that could satisfy it. The trigger is now *new* work, and the
work that predates the rule is recorded once —
`scripts/gate_debt_baseline.py` generated **39 pairs across 22 repositories**
into `gate-debt-baseline.json` from the repositories' own directories. That is a
committed list a reviewer reads, not a switch: no environment variable, no flag.
And a stale entry is dead weight rather than a live hole, because a repository
that discharges its debt leaves `gate-debt.json` and is never consulted again.

The exit is a second committed record, `gate-debt-exceptions.json`:
`(project, change, reason)`, in the shape of `rules-drift-baseline.json`. A
satisfied pair is reported at `WARNING` with its reason quoted, so a deliberate
decision stays visible in the run instead of becoming an invisible pass.

**The code was declared in `driftwatchdog` in the same change, after the false
green was reproduced rather than assumed.** On a fixture, before the
declaration:

```
governance-quality -> PASS   "0 of 93 governance findings matched this concern's scope"
```

— a checker that emitted the finding and exited 1, behind a green gate. After
adding it to `GOVERNANCE_QUALITY_DECLARED_CODES` and rebuilding:

```
governance-quality -> FAIL   "1 of 93"   rule QUALITY_NEW_CHANGE_WITH_GATE_DEBT
gate status FAIL, blocked: true, exit 1
```

Measured: `python3 -m unittest discover -s tests` **Ran 800 tests, OK**;
`pytest -q` **800 passed, 175 subtests**; `driftwatchdog` **909 passed, 0
failed** (908 before, +1 new test); `--self-check` **0 errors / 0 warnings**,
exit 0; the fleet audit is **279 errors / 20 warnings, none of them this rule's**.
The negative control is a test, not a claim: weakening the trigger to
grandfather every change turns 9 of the 36 new tests red, including the gate
acceptance, and restoring it turns them green.

The pre-existing failure this session started with,
`test_the_gate_is_open_and_the_fleet_corpus_is_clean`, was fixed by the owner's
concurrent commits `f192771` and `57dd605`, not here. Nothing in this change
narrows what that test checks. `openspec validate --all --strict` is now
**22 passed, 0 failed**; the 1 failure recorded here, `bind-declared-verification`,
was a stray untracked rollback record with no `specs/` deltas, resolved at
`9105360` — the change is `bind-declared-verification-command` in
`driftwatchdog`. The change is **not archived**; archiving needs separate
authorization.

## Follow-up: the recorded gate debt is cleared — 2026-10-06

The remaining 11 entries were resolved in the affected repositories, using
their existing `close-unbound-gate-checks` OpenSpecs. Each product repository
received the implementation/spec commit followed by a HANDOFF-only commit:

| Repository | Implementation | HANDOFF |
|---|---|---|
| `hestiaLab` | `d33995c` | `40d058f` |
| `jenkins-local` | `7b170e7` | `6fa8727` |
| `sisyphusfy` | `8e66b5d` | `75948c7` |
| `alethefy` | `4c97a73` | `f14d2c2` |
| `ariadex` | `07b93e3` | `e24e255` |
| `catchen` | `27c9cc9` | `d3b2bbf` |
| `linglitch` | `4bc4cef` | `567a045` |
| `platform-contracts` | `1c95354` | `2d62fce` |

Jenkins-local's build is Python adapter bytecode compilation; its README
assigns selected project image builds to the Mac release runtime. Sisyphusfy
builds its root Python distribution and audits its published npm launcher;
`.opencode/` package files are ignored local integration files, absent from
clean checkouts. Platform Contracts' schema and fixture wrappers pass with no
findings. The composite backend/frontend commands in Catchen and Linglitch
were bound but their full test suites were not executed in this pass.

The authoritative debt snapshot was regenerated at
`workspace-governance/gate-debt.json` with the explicit `--debt-out` path.
The driver dry-run now reports **0 entries and 0 decisions needed**. The
portfolio scan measured 81 repositories: 49 resolved, 0 unresolved, 5 without
manifests, and 27 with other dirty manifest work preserved. The unrelated
untracked `HANDOVER.md` and `driftwatch.toml` remain untouched. No changes were
archived or pushed.

Verification correction — Alethefy's build was subsequently run with
`UV_CACHE_DIR=/tmp/alethefy-uv-cache uv run --with build python -m build`.
uv selected Python 3.14.4, satisfying the package's `>=3.12` requirement; it
produced both sdist and wheel. The manifest now runs the build through uv, in
line with the repository workflow. Alethefy build verification is complete.
The manifest/spec update is `03d7d03`; its HANDOFF-only commit is `fc0c01e`.
Platform Contracts' `pip-audit` still could not establish an advisory result
through the restricted PyPI proxy, so that live result remains unverified.
Snapshot/task commit: `829c1be`.
