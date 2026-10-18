# Proposal: Make a README "not implemented" claim verifiable against the file tree

## Why

A README status line is the first thing an agent reads about a sibling
repository. **7 `AGENTS.md` files** across 6 repositories plus the project
template instruct exactly that: "Assess shared infrastructure before
duplication", "assess sibling `common`/`manager` projects". The assessment
has two inputs — the sibling's declared classification and the sibling's
README — and neither is currently checked against reality. Seven
repositories in this workspace state in prose that they contain no
implementation while their file tree contains one.

Measured 2026-10-03, bounded read-only, excluding `node_modules`, `dist`,
`build`, `target`, `bin`, `obj`, `vendor`, `coverage`, `.git`,
`__pycache__`, `.venv`, `.dart_tool`, `.next`, `out`, `.turbo`:

| Repository | README claim | What the tree holds | Verdict |
|---|---|---|---|
| `pluggify` | `README.md:9` "planning-only. No extension application has been implemented, packaged, published, archived, or released" | 112 `.ts` files, built `packages/shared/dist` | **contradicted** |
| `careBridge` | `README.md:17` "No application source, runtime, deployment, provider integration, or production evidence exists" | 32 `.cs`, `CareBridge.sln`, `src/CareBridge.Api/Program.cs`, `compose.yaml`, 7 pinned `Platform.*` packages | **contradicted** on 4 of the 5 surfaces it names |
| `testify` | `README.md:92` "contains specifications and documentation only" | 97 source files (86 `.cs`, 10 `.py`), `testify.sln`, `tests/` | **contradicted** |
| `aesthra` | `README.md:13` "has no application source, selected implementation stack, build, test, CI, or runtime verification" | 137 `.cs`, `Aesthra.sln`, `tests/`, `.github/workflows/governance.yml` | **contradicted** on 4 of 5 |
| `reelio` | `README.md:31` "contains no application source, tests, CI configuration, database migrations, or deployment evidence" | `src/api` (78 `.cs`) + `src/client` (9), `Reelio.Api/Program.cs`, `.github/workflows/ci.yml`, `Data/Migrations` | **contradicted** on 3 of 5 |
| `shopifireman` | `README.md:17` "contains planning artifacts only" | 115 source files, 770 KB, under `apps/` and `packages/` | **contradicted** |
| `rostera` | `README.md:44` "No source, build, test, deployment, provider, or runtime integration exists yet" | 354 `.cs` (1.5 MB) under `src/` and `tests/`, `Rostera.slnx`, `compose.yaml`, `Rostera.Api/Program.cs` + 2 controllers | **contradicted** on 5 of 6 |

The cost is not the seven wrong sentences. It is that each one is a
**false negative in the duplication assessment the workspace mandates**: an
agent told to assess a sibling before duplicating reads "no application
source exists" and correctly concludes there is nothing to reuse, then
writes its own. That is precisely the outcome
`capability-adoption-gate` measures across 34 idempotency, 28 rate-limiting
and 28 telemetry sites, arriving by a different route.

**The fix cannot be a keyword ban, and the corpus proves it.** Two
repositories in the same workspace carry negative status language and are
**not** wrong:

- `kairosa/README.md:11` — "Domain read models implemented in
  `src/Kairosa.CulturalData` … with 124 passing xUnit tests. … **No product
  runtime, data set, API, web client, mobile client, or production
  verification exists yet** — the library is an in-memory domain model."
  All six named surfaces are genuinely absent. The repository has a test
  suite and a CI workflow, and it never claims otherwise.
- `chronicleaf/README.md:19` — "What does not exist yet: GitHub
  integration, Evidence Bundles, screenshot capture, media storage, the
  credential vault, AI generation, Markdown drafts, review/approval, export
  and the Milestone Timeline." Ten named items, none of them a surface this
  checker can resolve, and it is a current, accurate status.

A check that reads "the README says nothing is implemented" and counts files
would fire on `kairosa`, the most honest status block in the workspace. A
check that reads the whole repository as one claim also fires on it. The
only rule that separates the seven from the two is **claim scope**: a claim
is verified only against the surfaces it names, and nothing else.

The natural home already exists. `scripts/product_code_quality.py` is a
bounded, read-only, fail-closed per-repository checker with
`DEFAULT_EXCLUDED_DIRS`, a per-language `LANGUAGE_EXTENSIONS` census, and a
`PASS | FAIL | REVIEW_REQUIRED` report that `workspace_check.py` already
translates into `QUALITY_*` findings. It has no rule for documentation
claims, and this change adds one.

## What Changes

- Add a **readme-status-claim rule** to `product_code_quality.py`: detect a
  present-tense absence claim in `README.md`, resolve each claim to the
  closed set of surfaces it names, and test each named surface against
  bounded, read-only file-tree evidence.
- Add a **closed surface vocabulary** shared with `vocabulary.json`:
  `application-source`, `build`, `test`, `ci`, `runtime`, `api`,
  `web-client`, `mobile-client`, `migrations`, `deployment`,
  `provider-integration`, `production-evidence`, plus the
  `repository-content` pseudo-surface that covers "planning artifacts
  only" and "specifications and documentation only". Every surface in the
  set is decidable from bounded file-tree evidence; a surface that is not is
  not a check and is not in the vocabulary.
- **A claim is verified only against the surfaces it names.** A claim that
  says nothing is implemented MUST NOT be evaluated against the whole
  repository, and a claim about a subset MUST NOT be evaluated against the
  rest. This is the rule that keeps `kairosa` and `chronicleaf` clean, and
  it is stated as a requirement rather than left to the implementation.
- **A claim that cannot be resolved is never a pass**, and is not a
  per-repository finding either. A negative status statement that names no
  surface from the closed vocabulary contributes to `QUALITY_README_CLAIM_UNSCOPED`,
  which is demoted out of the per-repository findings stream and emitted as
  a single aggregate CHECKER-COVERAGE metric — a claim the checker cannot
  test is reported as untested, never as true, and never as a defect
  attributed to a repository.
- **A claim contradicted by the tree is a finding**: ERROR
  `QUALITY_README_CLAIM_CONTRADICTED`, one per contradicted surface, naming
  the README line, the surface, and the evidence path that contradicts it.
  It ships together with the authorized repairs recorded below; the level is
  not negotiable while those repairs are outstanding.
- **Additive and claim-triggered, not opt-in gated.** The rule fires only
  when a qualifying claim exists, so the other 72 registered repositories
  produce exactly the findings they produced before. Only 2 of 81
  declarations carry a `quality` block, so gating the rule behind that block
  would leave 79 repositories unchecked; the check is therefore invoked for
  every registered project from `validate_project`, and the claim itself is
  the trigger.
- **No `readme_claims` block, no opt-in, no exemption.** The proposed
  additive `readme_claims` block is rejected. Every future claim is already
  checked in every repository today, so `{"mode": "check"}` would add a
  second, weaker path through the same check, and `{"mode": "off"}` would
  make the check switchable — the failure mode this workspace has already
  demonstrated. The schema gains no new field.
- **This change detects; the repairs are separately authorized.** Correcting
  a README is a source edit to a sibling repository, which `AGENTS.md`
  forbids without scope authorization, so this package writes no sibling.
  The seven repairs named below are authorized as seven separate,
  individually reviewable cross-repo source edits, one commit per repository,
  in the recorded blast-radius order. The finding names the line and the
  contradiction; the wording is the repository owner's, in a commit they
  review.

## Package Boundary and Split Assessment

This package ends at **a detected contradiction with a named evidence
path**. It does not edit a README, and it does not decide which of the two
sides of the contradiction is wrong — a stale claim and an undeclared
implementation are different problems with different owners, and the
checker can only see that they disagree.

| Package | Single outcome | Owner / language | Boundary and contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `readme-claim-verification` (this) | A "not implemented" README claim is checked against the file tree, and a contradiction is a finding | `workspace-governance`, Python 3.11 stdlib | `product_code_quality.py` rule `readme-status-claim`; `vocabulary.json` surface set; `QUALITY_README_*` findings | existing quality-checker harness; no sibling edit | the 9-repository measured corpus resolves to 7 contradicted / 2 clean; the 72-repo remainder is byte-identical |
| `identity-and-taxonomy-axis-split` (companion) | Immutable `id` plus two classification axes | `workspace-governance`, Python 3.11 stdlib | `vocabulary.json` `roles`/`stacks`; `TAXONOMY_*` findings | none from this change | independent; it never reads a README |
| `cross-repo-adoption-grant` (not this) | The lawful permission to repoint a consumer onto a provider | `workspace-governance`, Python 3.11 stdlib | `adoption_grant` declaration, `adoption/adoptions.json` | registry + provider identity | independent |

The two packages are not merged even though both edit
`vocabulary.json` and both run in the same audit, for one concrete reason:
their blast radii are opposite. Reclassifying a repository is a
governance-owned edit this owner may make unilaterally; rewriting a
sibling's README is a source edit to another repository's files. A single
package containing both would let a 9-file vocabulary change carry a
7-repository source edit, and the source edit would be the part that stalls.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `product_code_quality.py` | `Policy`, `evaluate`, `Report`, `DEFAULT_EXCLUDED_DIRS`, `LANGUAGE_EXTENSIONS`, `_candidate_files` | the bounded, read-only, fail-closed per-repo harness; already censuses file extensions and never executes a project | No rule inspects documentation; `parse_policy` requires a `quality` block that only 2 of 81 declarations carry | workspace-governance | **extend shared owner** — new rule, existing harness, no new process |
| `validate_product_quality` | `scripts/workspace_check.py:1629` | the in-process bridge; FAIL→ERROR, REVIEW_REQUIRED→WARNING, PASS→no finding | Only runs when `declaration["quality"]` is a dict, so it cannot host a claim-triggered rule | workspace-governance | **sibling of, not replacement for**: add `validate_readme_claims`, invoked unconditionally like `validate_kind` |
| `validate_kind` | `scripts/workspace_check.py:372` | precedent for auditing a descriptive declaration field for every project, not only opted-in ones | — | workspace-governance | **adopt the invocation pattern** for the unconditional call |
| `vocabulary.json` | `placeholder_markers`, `secret_field_substrings` | one machine-readable word list, loaded fail-closed by both scripts | Has no claim phrase or surface list | workspace-governance | **extend additively** (`readme_claim_surfaces`, `readme_absence_phrases`) |
| `discoverability` metadata | `scripts/repository_metadata.py`, `publication` evidence | README-derived metadata already exists for a different purpose: audience, claims, SEO | Reads README for *presence* of metadata; never cross-checks a status claim against the tree | workspace-governance | **reference, do not extend** — a different question about the same file |
| `standalone` classification | `classify_standalone` | precedent for a read-only tree classification that never overrides a declaration and never executes a project | Classifies capability to operate alone, not documentation truth | workspace-governance | **adopt the posture**: advisory, bounded, never a pass by omission |
| `AGENTS.md` duplication rule | 7 files, 6 repos + template | the instruction that makes the README consequential | The instruction has no defined evidence source for "does a sibling already do this" | each product | **document the finding in `docs/product-code-quality.md`**; no sibling edit in this change |
| `driftwatchdog` gate runtime | gate `checks:` binding | consumes governance findings as evidence | Owns command execution, not claim semantics | driftwatchdog | **defer** — the rule is a governance check, not a gate check |
| `forge_governance_adapter` | observation `findings[]` | passes new codes through unchanged | — | workspace-governance | **no change**; new codes appear only in the existing findings list |

No sibling repository owns documentation-truth verification, so no
extraction is authorized. `workspace-governance` owns it, and the
`product_code_quality.py` harness is the existing shared owner it extends.

## BFS Impact Map

- **Actors:** the governance auditor (read-only), a repository owner
  (decides whether the README or the tree is the thing that is wrong), an
  agent performing the mandated duplication assessment (consumer of the
  finding).
- **State:** a README claim moves `unexamined → verified-consistent` when
  every surface it names is absent, or `unexamined → contradicted` when at
  least one named surface is present. A negative status statement that
  names no known surface does not move per repository: it increments the
  aggregate CHECKER-COVERAGE metric and is owned by the checker's coverage,
  not by the repository.
- **Contracts:** `vocabulary.json` gains `readme_claim_surfaces` and
  `readme_absence_phrases`; `product_code_quality.py` gains rule
  `readme-status-claim`; `workspace_check.py` gains
  `QUALITY_README_CLAIM_CONTRADICTED` and the aggregate
  `QUALITY_README_CLAIM_UNSCOPED` coverage metric. The declaration schema
  gains nothing.
- **Local files:** `README.md` is **read only**. No sibling repository is
  written. The governance repository's own `README.md`,
  `docs/product-code-quality.md` and `HANDOFF.md` are the only files this
  change edits outside `scripts/`, `schemas/` and `vocabulary.json`.
- **Integrations:** the existing `findings[]` array in the Forge
  observation, the existing `QUALITY_*` translation, and the existing
  `maturity_audit.py` single-result invariant.
- **Failure and boundary:** a missing or unreadable `README.md` produces no
  claim and therefore no finding — absence of documentation is not a false
  claim, and the profile requirement `REQUIRED_FILE` already owns that case.
  A README over a bounded size, a claim inside a fenced code block, a claim
  in the future tense, and a claim naming an unknown surface are all
  non-qualifying and produce no contradicted finding. A claim that names no
  resolvable surface increments the aggregate CHECKER-COVERAGE metric
  instead of producing a per-repository finding — never a silent pass, and
  never a defect attributed to a repository.
- **Compatibility:** a repository with no qualifying claim produces exactly
  the findings it produced before this change. The 72 registered
  repositories outside the measured corpus are the regression set.
  `PASS`, `FAIL` and `REVIEW_REQUIRED` keep their meanings; exit codes 0/1/2
  are unchanged.
- **Security:** bounded reads of one file (`README.md`) plus a bounded
  directory walk using the checker's existing `DEFAULT_EXCLUDED_DIRS`. No
  project command, no package manager, no network, no symlink following, no
  write to any project. Finding messages carry a path and a line number, and
  are passed through the existing `redact` helper.
- **Verification:** fixtures for each of the 9 corpus repositories; a
  claim-scope fixture proving `kairosa`-shaped wording produces no finding;
  an unknown-surface fixture proving the aggregate coverage metric increments
  and no repository is flagged; a code-block and future-tense fixture; a
  fixture proving there is no per-repository opt-in — the rule runs for a
  repository that declares nothing — and a read-only proof (recursive tree
  hash identical before and after).
- **Dependencies:** Python 3.11 standard library only.

## Capabilities

- **New:** `readme-claim-verification`.
- **Modified:** `product-code-quality-boundary` — the checker's scope
  extends from product source to the repository's own status claim. Its
  existing requirements (declare product source boundaries, reject test code
  in product scope, enforce the placeholder threshold, emit bounded
  machine-readable findings) are preserved unchanged; the fail-closed
  posture and the `PASS | FAIL | REVIEW_REQUIRED` vocabulary are reused
  rather than redefined.
- **Unchanged:** `portfolio-vocabulary`. The new vocabulary arrays are
  additive and no existing array is narrowed.

## Non-goals

- No edit to any sibling repository's `README.md`, source, or CI *inside this
  package*. This change detects a contradiction and names it; the repository
  owner decides which side is wrong. The seven repairs recorded in ruling 4
  are authorized separately, one commit per repository, in the recorded
  order, and are not part of this package's diff.
- No automatic rewriting, deletion, or "suggested replacement" text for a
  README claim.
- No per-repository opt-in, off switch, exemption registry, or
  repository-specific surface extension. The proposed `readme_claims` block is
  rejected in full (ruling 5).
- No requirement that a README carry a status section, and no new required
  field in any schema. A repository with no claim is valid and silent.
- No judgement about a repository's `lifecycle`, `evidence_status`,
  `deployable` flag, promotion state, or capability adoption. A README is
  prose, and this repository's invariant holds: `workspace_check.py` does
  not promote a capability to verified on its own, and neither does a
  sentence in a README.
- No inference from a positive claim. "Five of 53 packages are implemented"
  is not checked; only present-tense absence claims are in scope.
- No merging of the claim phrase and surface lists into the schema; they
  live in `vocabulary.json` beside the other shared word lists, so one file
  owns the vocabulary.
- No new finding code for a missing or unreadable README.

## Owner rulings — decided

The owner has ruled on all five questions this section previously held open.
The rulings are binding on the finding levels, the surface vocabulary, the
schema, and the repair sequence. The corresponding record in design terms is
`design.md` §10.

1. **`QUALITY_README_CLAIM_CONTRADICTED` is ERROR.** There are 19 findings
   across 7 repositories, and a warning nobody must clear is precisely the
   mechanism by which those 7 stale claims accumulated in the first place.
   The level is not negotiable while the contradictions stand, and it ships
   together with the repair authorization in ruling 4 — the ERROR and the
   authorized repairs are one decision, not two.
2. **`QUALITY_README_CLAIM_UNSCOPED` does exist, but it is demoted out of the
   per-repository findings stream** and emitted as a single aggregate
   CHECKER-COVERAGE metric. It fires exactly once, on `chronicleaf`, whose
   ten-item negative list is accurate: a per-repository finding on an honest
   repository is a false positive, and false positives train people to ignore
   the checker. The information the code carries is about the checker's own
   coverage, not about a repository's defect, so it is reported where it
   belongs.
3. **`data-set` is dropped from the surface vocabulary.** The surface cannot
   be decided from a file tree with confidence — a repository can have
   `fixtures/` and no data set, or a `data/` directory of stubs — and a
   surface that cannot be decided is not a check. Retaining it would make the
   vocabulary overstate its own coverage. With it dropped, `kairosa`'s claim
   is knowingly unchecked on that axis: the phrase resolves to no surface and
   nothing is asserted about it. That is consistent with ruling 2 and honest
   in the direction that matters — the checker no longer implies it verified
   something it did not. The `kairosa` row in `design.md` §5 is updated so its
   expected outcome no longer claims coverage of that axis.
4. **Repair order for the 7 contradicted READMEs is blast radius descending,
   one commit per repository, each a separately authorized cross-repo source
   edit.** The order is fixed:

   | Order | Repository | Contradicted surfaces | Measured evidence |
   |---|---|---|---|
   | 1 | `rostera` | 5 | 354 `.cs` (1.5 MB), `Rostera.slnx`, `tests/`, `compose.yaml`, `Rostera.Api/Program.cs` + 2 controllers; `provider` is the one named surface not contradicted (no `Platform.*` pin) |
   | 2 | `careBridge` | 4 | denies platform integration while 7 `Platform.*` packages are pinned, alongside 32 `.cs`, `CareBridge.sln`, `src/CareBridge.Api/Program.cs` and `compose.yaml` |
   | 3 | `aesthra` | 4 | 137 `.cs`, `Aesthra.sln`, `tests/`, `.github/workflows/governance.yml` |
   | 4 | `reelio` | 3 | `src/api` 78 `.cs` + `src/client` 9, `.github/workflows/ci.yml`, `Data/Migrations` |
   | 5 | `pluggify` | 1 | 112 `.ts`, built `packages/shared/dist` |
   | 6 | `testify` | 1 | 97 source files (86 `.cs`, 10 `.py`), `testify.sln`, `tests/` |
   | 7 | `shopifireman` | 1 | 115 source files, 770 KB, under `apps/` and `packages/` |

   The rationale: the repositories with the most contradicted surfaces are the
   most misleading to anyone reasoning about the fleet, so they are repaired
   first; and each repair is its own reviewable cross-repo commit rather than
   one sweep, because a seven-repository diff is not reviewable.
5. **The proposed opt-in `readme_claims: {"mode": "check"}` is REJECTED, and
   so is the `{"mode": "off"}` exemption with it — the block is rejected in
   full.** Every future claim is already checked in every repository today, so
   the opt-in would add a second, weaker path through the same check, and
   this workspace's demonstrated failure mode is precisely checks that can be
   switched off. No opt-in, no exemption registry: the declaration schema
   gains no field, and the rule runs for every registered project.
