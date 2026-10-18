# Design: readme-claim-verification

## 1. Implementation boundary

**Repository:** `/home/paul/code/workspace-governance` — Python 3.11, standard
library only. **No sibling repository is written**, including the seven with
contradicted claims.

**Files to change:**

- `vocabulary.json` — add `readme_claim_surfaces` (the closed surface set)
  and `readme_absence_phrases` (the claim vocabulary). Both are non-empty,
  duplicate-free, and loaded through the existing fail-closed path.
- `scripts/vocabulary.py` — surface the two arrays.
- `scripts/product_code_quality.py` — add the `readme-status-claim` rule:
  claim detection (`_readme_claims`), surface resolution
  (`_claim_surfaces`), and bounded surface evidence (`_surface_evidence`),
  plus the report findings. Reuse `DEFAULT_EXCLUDED_DIRS`,
  `LANGUAGE_EXTENSIONS`, `_is_path_safe`, `_is_excluded` and `redact`.
- `scripts/workspace_check.py` — add `validate_readme_claims`, called from
  `validate_project` for every registered project; declare
  `QUALITY_README_CLAIM_CONTRADICTED` and `QUALITY_README_CLAIM_UNSCOPED`.
- `docs/product-code-quality.md` — the new rule, the surface table, the two
  codes, and the measured corpus.
- `README.md` — one short paragraph; link rather than restate.
- `tests/test_readme_claim_verification.py` (new).

**Must not change:** every sibling repository; the declaration schema
`schemas/project.schema.json`, which gains no `readme_claims` block (§10
ruling 5); `capabilities.json`,
`decisions.json`, `projects.json`, `deployment/promotions.json`,
`scripts/promote_project.py`, the `quality` block's existing fields and
their validation, the `standalone` and `release_evidence` blocks,
`scripts/forge_governance_adapter.py`, and every existing
`placeholder_markers` / `secret_field_substrings` entry.

## 2. Language and runtime

Python **3.11**, standard library only.

- `python3 -m unittest discover -s tests -v`
- `python3 scripts/product_code_quality.py <project_dir> --json`
- `python3 scripts/workspace_check.py --root .. --audit --json`
- `python3 scripts/workspace_check.py --self-check`
- `node scripts/check-openspec-change-names.mjs`
- `openspec validate readme-claim-verification --strict`
- `git diff --check`

## 3. Ownership and shared code

- **Owner:** `workspace-governance`. `product_code_quality.py` is the
  existing shared owner of bounded, per-repository, read-only checks; this
  change adds a rule to it rather than creating a second checker.
- **Exported contract:** `vocabulary.json`
  `readme_claim_surfaces` and `readme_absence_phrases`.
- **Consumers:** `product_code_quality.py` (rule),
  `workspace_check.py` (finding translation), `driftwatchdog` when the
  governance check is bound as a gate check (deferred to that repository),
  and the Forge adapter through the unchanged `findings[]` array.
- **Dependency direction:** products → governance. The checker reads a
  sibling's files and writes none.
- **Release boundary:** ships with this repository.

## 4. The claim model

### 4.1 Claim detection

A **qualifying claim** is a sentence in `README.md` that satisfies all four
conditions:

1. it asserts **absence in the present tense of the repository's current
   state** — matched from `readme_absence_phrases` (`no application source`,
   `no source`, `nothing is implemented`, `not implemented`,
   `planning-only`, `planning artifacts only`, `specifications and
   documentation only`, `documentation only`, `does not exist yet`,
   `no … exists`, `remains to be built`);
2. it is **not** inside a fenced code block, an HTML comment, or a block
   quote;
3. it is **not** scoped to a future or hypothetical state (`the first
   release will be`, `once the solution exists, we will`,
   `is intended to`, `are planning decisions`);
4. it **names at least one surface** from `readme_claim_surfaces`, using
   `readme_absence_phrases`'s surface synonyms (§4.2).

Condition 4 is what separates a checkable claim from prose. A qualifying
claim resolves to a **set of surfaces**, and that set is the entire scope of
the verification. A claim that names six surfaces is tested on those six and
on nothing else. A claim that names none is not a claim about the
repository's implementation state and produces no contradicted finding.

### 4.2 Surfaces and their evidence

| Surface | Claim synonyms | Bounded evidence (read-only) |
|---|---|---|
| `application-source` | application source, extension application, source, implementation | ≥1 file with an extension in `LANGUAGE_EXTENSIONS` under a source root (`src`, `lib`, `app`, `apps`, `packages`, `tools`, `cmd`, `internal`, `server`, `client`) or the repository root, excluding `DEFAULT_EXCLUDED_DIRS` |
| `repository-content` | planning artifacts only, specifications and documentation only, documentation only, nothing is implemented | the same census as `application-source`, but not limited to a source root — a document-only tree has no source root at all |
| `build` | build | ≥1 `*.sln`, `*.slnx`, `*.csproj`, `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `pubspec.yaml`, or `CMakeLists.txt` |
| `test` | tests, test suite, test project | a `tests`, `test`, `__tests__` or `spec` root, a `*.Tests/*.csproj`, a `tests` target in a solution, or a file matching a language's test pattern |
| `ci` | CI, CI configuration, CI workflow, pipeline | `.github/workflows/`, `Jenkinsfile`, `.gitlab-ci.yml`, `azure-pipelines.yml`, `.circleci/` |
| `runtime` | runtime, product runtime, runtime integration, running application | a runnable entry point (`Program.cs`, `main.py`, `index.ts`, `main.go`, `cmd/`, `bin/`, an `ENTRYPOINT`/`CMD` in a `Dockerfile`) |
| `api` | API, HTTP API, endpoints | a `*Controller.cs`, `routes/`, `api/`, `openapi*.y(a)ml`, `swagger*.y(a)ml`, or a framework entry point that serves HTTP |
| `web-client` | web client, web UI, frontend, SPA | a `client`, `web`, `frontend`, `wwwroot` or `ui` root, or a `package.json` declaring a UI framework |
| `mobile-client` | mobile client, mobile app, Flutter client | `pubspec.yaml` with a Flutter SDK constraint, `*.xcodeproj`/`*.xcworkspace`, an `android`+`ios` pair, or `*.dart` outside `packages` |
| `migrations` | database migrations, migrations | a `migrations`, `Migrations`, `alembic` or `db/migrate` root, or `*.sql` inside one |
| `deployment` | deployment, deployment evidence | a `compose*.y(a)ml` at any depth, a `deployment/` root with a compose or template, or a `deployment.compose_file` in the declaration that resolves |
| `provider-integration` | provider integration, provider, sibling platform integration | a `capabilities[].consumes[]` record, or a pinned `Platform.*` / sibling package in `Directory.Packages.props`, `*.csproj`, `package.json`, `Cargo.toml` or `pyproject.toml` — reusing `read_manifest_dependencies` |
| `production-evidence` | production evidence, production verification, release evidence, published, released | a `release_evidence` block in the declaration, or a matching promotion-ledger entry |

`*.slnx` is in the `build` row because `rostera` ships `Rostera.slnx`; a rule
that recognised only `*.sln` would have missed one of the five surfaces that
contradicts that repository's claim.

### 4.3 The rule's control flow

```
read README.md
  -> extract qualifying claims (§4.1)
  -> for each claim, resolve its named surfaces (§4.2)
  -> for each named surface, evaluate bounded evidence
       present  -> QUALITY_README_CLAIM_CONTRADICTED  (ERROR,   one per surface)
       absent   -> no finding for that surface
       unknown  -> CHECKER-COVERAGE aggregate          (no per-repository finding)
  -> if no qualifying claim and a non-qualifying negative status statement exists
       -> CHECKER-COVERAGE aggregate                    (no per-repository finding)
```

One finding per contradicted surface, not per claim, because the remediation
differs per surface: `rostera` needs its claim corrected for source, build,
test, deployment and runtime, and a single "README is wrong" finding would
hide that. Each message names the README line number, the surface, and the
evidence path that contradicts it.

There is no per-repository opt-in and no exemption. The rule runs for every
registered project, nothing in a declaration can switch it off, and the
closed vocabulary is not extensible per repository (§10 ruling 5). The claim
is the only trigger.

## 5. The measured corpus

Nine repositories in the workspace carry negative status language. This is
the regression fixture, with the outcome this rule produces. It was measured
on 2026-10-03; the numbers are a bounded extension census, not a build.

| Repository | Claim line | Surfaces named | Evidence found | Expected outcome |
|---|---|---|---|---|
| `pluggify` | 9 | `application-source` | 112 `.ts`, `packages/shared/dist` | **ERROR** ×1 |
| `careBridge` | 17 | `application-source`, `runtime`, `deployment`, `provider-integration`, `production-evidence` | 32 `.cs` + `CareBridge.sln`; `src/CareBridge.Api/Program.cs`; `compose.yaml`; 7 pinned `Platform.*`; no `release_evidence` | **ERROR** ×4 |
| `testify` | 92 | `repository-content` | 97 source files (86 `.cs`, 10 `.py`), `testify.sln`, `tests/` | **ERROR** ×1 |
| `aesthra` | 13 | `application-source`, `build`, `test`, `ci`, `runtime` | 137 `.cs`; `Aesthra.sln`; `tests/`; `.github/workflows/governance.yml`; no entry point | **ERROR** ×4 |
| `reelio` | 31 | `application-source`, `test`, `ci`, `migrations`, `deployment` | `src/api` 78 `.cs` + `src/client` 9; `Reelio.Api/Program.cs`; `.github/workflows/ci.yml`; `Data/Migrations`; no tests root, no compose | **ERROR** ×3 |
| `shopifireman` | 17 | `repository-content` | 115 source files, 770 KB, under `apps/` and `packages/` | **ERROR** ×1 |
| `rostera` | 44 | `source`, `build`, `test`, `deployment`, `provider`, `runtime` | 354 `.cs` (1.5 MB); `Rostera.slnx`; `tests/`; `compose.yaml`; `Rostera.Api/Program.cs` + 2 controllers; no `Platform.*` pin | **ERROR** ×5 |
| `kairosa` | 11 | `runtime`, `api`, `web-client`, `mobile-client`, `production-evidence`; its `data set` phrase is **not** a surface (§10 ruling 3) | **none of the five present** — no entry point, no controller, no client, no data, no `release_evidence`. It has `tests/` and a CI workflow and claims neither | **no finding** — the `data set` claim is knowingly unchecked, not verified |
| `chronicleaf` | 19 | none in the closed vocabulary (`GitHub integration`, `Evidence Bundles`, `screenshot capture`, `media storage`, `credential vault`, `AI generation`, `Markdown drafts`, `review/approval`, `export`, `Milestone Timeline`) | n/a | **no per-repository finding** — counts 1 in the aggregate `QUALITY_README_CLAIM_UNSCOPED` CHECKER-COVERAGE metric (§10 ruling 2) |

Totals: **7 repositories contradicted, 19 contradicted surfaces, 0
per-repository unscoped findings, 2 clean**, plus **1 aggregate
CHECKER-COVERAGE gap** — `chronicleaf`'s unresolvable negative list, which is
about the checker's coverage and not about a repository. The remaining 72
registered repositories must produce exactly the findings they produced
before.

### 5.1 Two corpus facts that contradict the commissioning brief

Recorded here because the rule must be correct regardless of which
expectation is right, and because an implementer will hit both immediately.

1. **`chronicleaf` does not make a false claim.** Its `## Status` section
   states that five of 53 packages "are implemented, verified and archived",
   lists what the running system does today, and lists ten things that "does
   not exist yet". It is the most accurate status block in the workspace,
   and it is one of the four unregistered repositories. The rule must not
   fire `QUALITY_README_CLAIM_CONTRADICTED` on it.
2. **`reelio`, `shopifireman` and `rostera` are not honest stubs.** They were
   expected to be the must-not-fire set. They are not: `reelio` holds 87
   source files (1.17 MB) in `src/api` and `src/client` while its status line
   says it "contains no application source"; `shopifireman` holds 115 source
   files (770 KB) under `apps/` and `packages/` while it says it "contains
   planning artifacts only"; `rostera` holds 354 `.cs` files under `src/` and
   `tests/` while it says "No source, build, test, deployment … exists yet".
   All three claims are as stale as the seven. The genuinely honest
   must-not-fire cases are `kairosa` and `chronicleaf`, and both are honest
   for different reasons — `kairosa` because its claim is *scoped* to six
   surfaces that are genuinely absent, `chronicleaf` because it makes no
   checkable absence claim at all.

`kairosa` is the case that decides the design. Its repository contains 22
`.cs` files and 124 passing xUnit tests, and its status line is accurate,
because the line scopes its absence to *product runtime, data set, API, web
client, mobile client and production verification* — six surfaces, all
absent. A rule that evaluated "the README says something is not implemented"
against a repository-wide file count would fire on it and would be wrong.
Claim scope is therefore a requirement, not an implementation detail.

## 6. Contract and compatibility

- `vocabulary.json` gains `readme_claim_surfaces` and
  `readme_absence_phrases`. Both are non-empty and duplicate-free;
  `schema_version` stays `1` and no existing array changes.
- New codes, extending the existing `QUALITY_*` family used by
  `validate_product_quality`. Neither collides with a code already in use
  (`QUALITY_POLICY`, `QUALITY_TEST_CODE`, `QUALITY_PLACEHOLDER`,
  `QUALITY_FAILURE`, `QUALITY_REVIEW_REQUIRED`):

| Code | Level | Fires when |
|---|---|---|
| `QUALITY_README_CLAIM_CONTRADICTED` | ERROR | a surface named by a qualifying README claim is present in the file tree; one per surface, naming line, surface, and evidence path |
| `QUALITY_README_CLAIM_UNSCOPED` | aggregate CHECKER-COVERAGE metric, not a per-repository finding | a negative status statement names no surface the checker can resolve; counted once across the fleet, never attributed to a repository, and never a pass |

- The rule id inside the checker is `readme-status-claim`, following the
  existing `product-test-code` and `placeholder-threshold` convention.
- `PASS`, `FAIL` and `REVIEW_REQUIRED` keep their meanings; exit codes 0/1/2
  are unchanged; the report schema is unchanged. `FAIL` promotes to ERROR
  and `REVIEW_REQUIRED` to WARNING through the same translation the existing
  rules use.
- A README is prose. This change MUST NOT alter any `lifecycle`,
  `evidence_status`, `deployable` flag, promotion-ledger entry, capability
  adoption posture, or `verification.evidence_status`. The documented
  invariant is preserved verbatim: `workspace_check.py` does not promote a
  capability to verified on its own, and a sentence in a README does not
  either.
- The existing `quality` block is untouched. A repository with no `quality`
  block is still checked, because the claim is the trigger — only 2 of 81
  declarations carry a `quality` block, so gating on it would leave 79
  repositories unexamined.

## 7. Failure and boundary policy

| Case | Result |
|---|---|
| `README.md` absent | no claim, no finding. `REQUIRED_FILE` owns the missing-file case for profiles that require it |
| `README.md` unreadable or not valid UTF-8 | `QUALITY_README_CLAIM_UNSCOPED` WARNING naming the file; never a pass, never an ERROR the owner cannot act on |
| `README.md` above the bounded read size (64 KiB) | the first 64 KiB is read and the finding message states that the file was truncated; a claim in the tail produces no finding, and that absence is reported as `QUALITY_README_CLAIM_UNSCOPED` |
| claim inside a fenced code block, HTML comment, or block quote | not a claim; no finding |
| claim in the future tense or scoped to a release that does not exist yet | not a claim; no finding. This is what keeps `rostera/README.md:20` out and lets `:44` carry the finding |
| claim names a surface that is not in the vocabulary | no contradicted finding for it; the gap is counted in the aggregate CHECKER-COVERAGE metric. The vocabulary is never widened to admit a surface that cannot be decided from the tree (§10 ruling 3) |
| claim names only unknown surfaces (`chronicleaf`) | no per-repository finding; the claim counts 1 in the aggregate CHECKER-COVERAGE metric, because the repository's claim is accurate and the gap is the checker's (§10 ruling 2) |
| claim names a surface that is absent | no finding for that surface; the claim is verified as far as it goes |
| two claims in one README disagree with each other | both are evaluated; each contradicted surface is reported once, against the first claim that names it, and the message names both line numbers |
| a declaration carries a `readme_claims` block | no such field exists; the rule runs for that repository exactly as for every other, and an unknown key is a schema violation like any other |
| a path escape (`..`) or absolute path in any configuration | refused by the existing `_is_path_safe`; never normalized |
| a symlinked directory in the walk | not followed, matching `discover_projects` |
| `vocabulary.json` missing, unparseable, or with an empty surface list | hard non-zero exit naming the file; an empty surface list would make every claim vacuously true |

## 8. Verification oracle

- `python3 -m unittest discover -s tests -v` — all pass, including the nine
  corpus fixtures in `tests/test_readme_claim_verification.py`, and no
  existing test changes expectation.
- **Corpus assertion:** the nine-repository fixture set produces exactly
  7 contradicted projects, 19 contradicted surfaces, 0 per-repository
  `QUALITY_README_CLAIM_UNSCOPED` findings with 1 aggregate coverage gap, and
  2 clean projects, matching §5.
- **Must-not-fire assertion:** `kairosa` and `chronicleaf` produce no
  `QUALITY_README_CLAIM_CONTRADICTED`, and the `kairosa` fixture passes even
  though its repository contains source files and a test suite.
- **Additivity assertion:** the other 72 registered repositories produce a
  finding set identical to the pre-change baseline.
- **Read-only proof:** a recursive tree hash of a fixture project is
  identical before and after, and no child process is spawned.
- `python3 scripts/product_code_quality.py <project_dir> --json` — one
  parseable report, correct exit code, and `redact` applied to every message.
- `python3 scripts/workspace_check.py --root .. --audit --json` — 0 errors
  after the seven repositories have corrected their claims; the
  pre-correction output showing 7 contradicted projects is recorded, not
  hidden.
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings
  for the governance repository itself.
- `node scripts/check-openspec-change-names.mjs`,
  `openspec validate readme-claim-verification --strict`, `git diff --check`.
- Record actual outputs. Any environment-blocked check is reported with its
  exact next action, never as passing.

## 9. Decision ledger

**Assumptions:** a README is prose and is never edited by this repository; a
claim is verified only against the surfaces it names; a claim that cannot be
evaluated is reported as untested rather than assumed true; the tree is
stronger evidence than prose for "does source exist", and weaker for
everything else.

**Resolved alternatives:**

- (a) A new checker vs a rule in `product_code_quality.py` → chose
  **a rule in the existing checker**. It already has the bounded walk, the
  excluded-directory set, the extension census, the fail-closed report
  vocabulary, and the in-process bridge. A second checker would duplicate all
  of it.
- (b) Gate the rule behind the `quality` block → chose **not to gate it**.
  Only 2 of 81 declarations carry that block; gating would leave 79
  repositories unchecked and the rule effectively dead. The claim is the
  trigger, and the invocation follows the unconditional `validate_kind`
  precedent.
- (c) Repository-wide file count vs claim-scoped surface evidence → chose
  **claim-scoped**. Measured: `kairosa` has 39 source files, a test suite
  and a CI workflow and its status line is accurate, because it scopes its
  absence to six absent surfaces. A repository-wide rule fires on the most
  honest status block in the workspace.
- (d) Auto-rewrite the README → chose **detect only**. Rewriting a sibling's
  prose is a cross-repository source edit that `AGENTS.md` forbids without
  authorization, and the checker cannot know which side of the contradiction
  is wrong.
- (e) Drop an unverifiable claim silently vs report it → chose
  **report it**, as `QUALITY_README_CLAIM_UNSCOPED` demoted by owner ruling
  to a single aggregate CHECKER-COVERAGE metric rather than a
  per-repository finding (§10 ruling 2). Silence would make an untestable
  claim indistinguishable from a verified one, and a per-repository finding
  on an honest repository would be a false positive that trains people to
  ignore the checker. Both failure modes are removed by reporting it
  against the checker's coverage.
- (f) One finding per claim vs per contradicted surface → chose **per
  surface**. `rostera` contradicts five surfaces with five different
  remediations; one finding would hide four of them.
- (g) Merge with `identity-and-taxonomy-axis-split` → chose **separate**;
  see the package boundary table.

**Deferred:** binding the rule
as a `driftwatchdog` gate check (owned by that repository); reporting the
contradiction count per repository in `maturity_audit.py` (display only, and
only after the seven are repaired); and any automated suggestion of
replacement README wording.

**Blockers:** none. The five open decisions are now owner rulings (§10) and
none of them changes the rule's detection logic. Ruling 4's repair order is a
cross-repo source-edit sequence outside this package: this change writes no
sibling, and each of the seven repairs is separately authorized and reviewed
on its own.

## 10. Owner rulings — decided

The owner has ruled on the five questions this change held open. The rulings
are binding on the finding levels, the closed surface vocabulary, the
declaration schema, and the repair sequence. The rationale is recorded in
`proposal.md` "Owner rulings — decided"; this section records the same
rulings in design terms.

| Ruling | Decision | Effect on this design |
|---|---|---|
| 1 | `QUALITY_README_CLAIM_CONTRADICTED` is ERROR | §4.3, §6 and §7 keep ERROR, one finding per contradicted surface. The level ships together with the authorized repairs in ruling 4; it is not a ramp. |
| 2 | `QUALITY_README_CLAIM_UNSCOPED` exists, demoted to an aggregate CHECKER-COVERAGE metric | §4.3's `unknown` branch, §6's code table and §7's `chronicleaf` row count the gap once across the fleet and emit **no per-repository finding**. §5's totals and §8's corpus assertion state 0 per-repository findings and 1 coverage gap. The file-level triggers (unreadable, over-size `README.md`) keep the same code and the same WARNING level: they describe a file the checker could not read, not a claim it could not scope, and no corpus repository exercises them. |
| 3 | `data-set` is dropped from the surface vocabulary | The `data-set` row is removed from §4.2, and §7 states that the vocabulary is never widened to admit a surface that cannot be decided from the tree. `kairosa`'s §5 row drops `data-set` from its named surfaces and its expected outcome now reads "knowingly unchecked, not verified": the checker asserts nothing about that axis, consistent with ruling 2. |
| 4 | Repairs run blast radius descending, one authorized commit per repository | `rostera` (5) → `careBridge` (4) → `aesthra` (4) → `reelio` (3) → `pluggify` (1) → `testify` (1) → `shopifireman` (1). §1's boundary is unchanged: this package writes no sibling, and each repair is a separate cross-repo commit outside this diff. §8's "0 errors after the seven repositories have corrected their claims" is the end state of that sequence. |
| 5 | The `readme_claims` block is rejected | No schema change (§1 moves `schemas/project.schema.json` to *must not change*), no `{"mode": "off"}` exemption, no `{"mode": "check"}` opt-in, no per-repository surface extension. §4.3 has no configuration branch, and §7's configuration row now records that an unknown `readme_claims` key is a schema violation like any other. The rule is invoked for every registered project from `validate_project` and cannot be switched off. |

**Conflicts recorded, not silently resolved.**

- `specs/readme-claim-verification/spec.md` still asserts the pre-ruling
  contract in four places, and it was **not** edited in this pass: the
  requirement "A claim that cannot be resolved is reported, never passed"
  still requires a WARNING `QUALITY_README_CLAIM_UNSCOPED` per project; its
  scenario "A claim names an undecidable surface" still names `data-set`; and
  the requirement "The claim check is additive and claim-triggered" still
  allows `readme_claims` with `mode` `off` or `check`, with three scenarios
  built on them — including an `audit` mode that appears nowhere in this
  design. Those scenarios contradict rulings 2, 3 and 5 and must be
  rewritten before implementation; the spec is the contract and the ruling
  does not silently rewrite it.
- `tasks.md` §0 still asks for the level to be decided first and §2.1 still
  instructs the implementer to include `data-set` "so it reports untestable
  rather than unchecked". Both are superseded by rulings 1 and 3 and need a
  follow-up edit outside this pass.
- Ruling 3 means `kairosa`'s six-part claim is verified on five surfaces and
  unchecked on one. That is a deliberate reduction in coverage, taken so the
  vocabulary does not overstate what the checker can decide. It is recorded
  here so the §5 row is never read as "six surfaces verified".

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| R1 A status claim is detectable | qualifying-claim definition | §4.1 | `product_code_quality.py` | present tense / future tense / code block | 2.1, 2.2 | claim fixtures |
| R2 A claim is verified against the file tree | surface evidence rules | §4.2, §4.3 | `product_code_quality.py` | surface present / absent / unknown | 2.3 | corpus: 19 contradicted surfaces |
| R3 Claim scope is respected | only named surfaces are tested | §4.1, §5.1 | `product_code_quality.py` | `kairosa` clean / subset claim | 2.3, 3.2 | `kairosa` fixture has no contradicted finding |
| R4 A contradiction is a finding | `QUALITY_README_CLAIM_CONTRADICTED` | §6, §7 | `validate_readme_claims` | per-surface finding / first line wins | 2.4, 2.5 | 7 contradicted projects |
| R5 An unverifiable claim is not a pass | `QUALITY_README_CLAIM_UNSCOPED`, aggregate | §4.3, §7, §10 | `validate_readme_claims` | unknown surface / unreadable / truncated | 2.5 | 0 per-repository findings, 1 aggregate coverage gap |
| R6 The rule is additive and claim-triggered | 72-silent baseline | §4.3, §6, §10 | `validate_project` only — no declaration field | no claim / no opt-in exists / unknown `readme_claims` key | 2.6, 3.1 | identical findings for 72 repositories |
| R7 Documentation claims never change evidence | prose is not evidence | §6 | none — invariant | contradicted claim with `deployable: true` | 3.4 | no evidence-state change |
