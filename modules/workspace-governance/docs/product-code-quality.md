# Product-Code Quality Gate

The product-code quality gate is a portable, deterministic repository
checker owned by Workspace Governance. It detects two failure modes that
the existing structural audit cannot:

- **Test code in product scope.** Code that ships in the product
  source root but is really a test. For Rust the check is lexical:
  `#[test]` or `mod tests` outside a `#[cfg(test)]` guard is rejected.
  Python, TypeScript, C#, and Go use the recognised file/import patterns
  of each language.
- **Placeholder debt in product scope.** Maintainer-visible markers
  such as `TODO`, `FIXME`, `XXX`, `HACK`, `placeholder`, `stub`,
  `unimplemented!`, and `todo!` are counted across product files. The
  count is compared against a configured threshold; the default for
  newly adopted product roots is **zero**.
- **A contradicted README status claim.** The `readme-status-claim`
  rule checks a present-tense "nothing is implemented here" statement
  against the file tree, on the surfaces the statement names. This one
  rule is **not** opt-in — see [README status claims](#readme-status-claims).

The gate is fail-closed: a missing or invalid `quality` block, an
unsupported language, an exclusion that escapes the project root, or an
unreadable product file is `REVIEW_REQUIRED`, never a guessed pass.

## How a project opts in

Add a `quality` block to the project's `.project.json`:

```json
{
  "quality": {
    "product_roots": ["src"],
    "languages": ["rust"],
    "exclude": ["src/generated", "src/fixtures"],
    "placeholder_threshold": 0
  }
}
```

| Field                  | Required | Meaning                                                                                                  |
|------------------------|----------|----------------------------------------------------------------------------------------------------------|
| `product_roots`        | yes      | Repository-relative directories that count as product source. Each must exist and be a directory.        |
| `languages`            | yes      | One or more of `rust`, `python`, `typescript`, `javascript`, `csharp`, `go`, `generic`.                  |
| `exclude`              | no       | Repository-relative paths under any product root to exclude. Must not escape the project root.            |
| `placeholder_threshold`| no       | Integer count of tolerated markers across product files. Default `0`.                                    |
| `placeholder_markers`  | no       | Override the default marker vocabulary (`todo`, `fixme`, `xxx`, `hack`, `unimplemented!`, `todo!`). Words are matched case-insensitively with word boundaries. |

## Running the checker directly

The checker is a normal command; Driftwatchdog binds it as a Gate check.

```bash
python3 scripts/product_code_quality.py <project_dir>            # human output
python3 scripts/product_code_quality.py <project_dir> --json     # versioned JSON report
python3 scripts/product_code_quality.py <project_dir> --policy <path>
```

Exit codes:

| Code | Meaning                                                                                          |
|------|--------------------------------------------------------------------------------------------------|
| `0`  | PASS — no blocking finding, no review-required gap.                                              |
| `1`  | FAIL — test code in product scope, or the placeholder count exceeds the configured threshold.    |
| `2`  | REVIEW_REQUIRED — missing or invalid policy, unreadable product file, or any other manual-review gap. |

## The JSON contract

```json
{
  "schema_version": 1,
  "status": "PASS | FAIL | REVIEW_REQUIRED",
  "project": "project-name",
  "summary": {
    "test_code": 0,
    "placeholder": 0,
    "errors": 0,
    "warnings": 0,
    "files_scanned": 0,
    "files_unreadable": 0,
    "excluded_paths": 0
  },
  "findings": [
    {
      "rule_id": "product-test-code | placeholder-marker",
      "level": "ERROR | WARNING",
      "severity": "FAIL",
      "file": "src/lib.rs",
      "line": 4,
      "marker": "#[test]",
      "remediation": "..."
    }
  ],
  "notes": ["..."]
}
```

## Audit integration

When a project declaration has a `quality` block, the workspace audit
(`workspace_check.py`) imports the checker in-process and translates
the report into its existing Finding vocabulary:

- `FAIL` → `ERROR` (codes `QUALITY_TEST_CODE`, `QUALITY_PLACEHOLDER`,
  `QUALITY_FAILURE`)
- `REVIEW_REQUIRED` → `WARNING` (codes `QUALITY_REVIEW_REQUIRED`,
  `QUALITY_POLICY`)
- `PASS` → no finding

Projects that omit the `quality` block continue to pass unchanged; the
rule is opt-in.

The `readme-status-claim` rule is the one exception, and it is described
next.

## README status claims

A README status line is a factual assertion, and it is the first thing
an agent reads about a sibling repository. The `readme-status-claim`
rule resolves a present-tense absence claim to the closed set of
surfaces it names, then decides each of those surfaces from bounded,
read-only file-tree evidence.

### Claim scope is the rule

**A claim is verified only against the surfaces it names, and against
nothing else.** A claim that names five surfaces is tested on those five.
A repository-wide file count is not a rule and is not used: a repository
can hold a domain library, a test suite and a CI workflow and still
carry an accurate status line, because the line scopes its absence to
surfaces that are genuinely absent. `kairosa` is that case and it is the
reason the scope requirement exists.

### The closed surface vocabulary

`vocabulary.json` carries two additive arrays — `readme_claim_surfaces`
and `readme_absence_phrases`. The surface keys are the closed set; the
synonyms and the evidence rules live in `product_code_quality.py` beside
the checker's other tables. The vocabulary is never widened per
repository, and never widened at all to admit a surface no file-tree
rule can decide (`data-set` was dropped for exactly that reason).

| Surface | Claim synonyms | Bounded, read-only evidence |
|---|---|---|
| `application-source` | no application source, no extension application, application source, extension application, no source | a language source file under a source root (`src`, `lib`, `app`, `apps`, `packages`, `tools`, `cmd`, `internal`, `server`, `client`) or the repository root, minus test/fixture/generated paths |
| `repository-content` | planning artifacts only, specifications and documentation only, documentation only, nothing is implemented | the same census, not restricted to a source root |
| `build` | build | a `*.sln`, `*.slnx`, `*.csproj`, `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `pubspec.yaml`, or `CMakeLists.txt` |
| `test` | tests, test suite, test project | a `tests`/`test`/`__tests__`/`spec` root, a `*.Tests/*.csproj`, or a file matching a language test pattern |
| `ci` | CI, CI configuration, CI workflow, pipeline | `.github/workflows/`, `Jenkinsfile`, `.gitlab-ci.yml`, `azure-pipelines.yml`, `.circleci/` |
| `runtime` | runtime, product runtime, runtime integration, running application | an entry point (`Program.cs`, `main.py`, `index.ts`, `main.go`, `main.dart`), a `cmd/` root, or an `ENTRYPOINT`/`CMD` in a Dockerfile |
| `api` | API, HTTP API, endpoints | a `*Controller.cs`, a `routes/` or `api/` root, or an `openapi*`/`swagger*` document |
| `web-client` | web client, web UI, frontend, SPA | a `client`/`web`/`frontend`/`wwwroot`/`ui` root, or a `package.json` declaring a UI framework |
| `mobile-client` | mobile client, mobile app, Flutter client | a Flutter `pubspec.yaml`, an Apple project/workspace, an `android`+`ios` pair, or a `*.dart` file outside `packages/` |
| `migrations` | database migrations, migrations | a `migrations`/`Migrations`/`alembic` root, or a `*.sql` inside one |
| `deployment` | deployment, deployment evidence | a repository declaring `deployment.deployable: false` has none — that declaration is consulted first. Otherwise a `compose*.y(a)ml` at any depth, a `deployment/` root, or a declared `deployment.compose_file` that resolves |
| `provider-integration` | provider integration, provider, sibling platform integration | a `capabilities[].consumes[]` record, or a pinned `Platform.*` package in a manifest |
| `production-evidence` | production evidence, production verification, release evidence, published, released | a `release_evidence` block, or a `promote` entry in the promotion ledger |

### The two codes

| Code | Level | Fires when |
|---|---|---|
| `QUALITY_README_CLAIM_CONTRADICTED` | `ERROR` | a surface named by a qualifying claim is present in the tree. One finding per contradicted surface, naming the README line(s), the surface, and the evidence path. |
| `QUALITY_README_CLAIM_UNSCOPED` | `WARNING`, fleet-level | a negative status statement names no surface this check can resolve. Emitted **once per run**, naming every unscoped statement and its repository. `REVIEW_REQUIRED` in the checker's own vocabulary; never a pass, and never attributed to a repository. |

`QUALITY_README_CLAIM_UNSCOPED` is not a per-repository finding on
purpose. It fires on repositories whose claims are **accurate** —
`chronicleaf`'s ten-item negative list is current — and a per-repository
finding on an honest repository is a false positive that teaches people
to ignore the checker. The information it carries belongs to the
checker's own coverage, so it is reported against the checker.

### There is no opt-in

The rule runs for **every** registered project and is triggered by claim
presence. There is no `readme_claims` block, no `mode`, no opt-out and
no per-repository exemption, and the declaration schema gains no field.
A declaration that carries a `readme_claims` key is reported as
`WARNING QUALITY_REVIEW_REQUIRED` naming the unsupported field, and the
canonical behaviour applies anyway. A check that can be switched off is
not a check.

Only 2 of 81 declarations carry a `quality` block, so gating the rule
behind it would have left 79 repositories unexamined.

### Bounded and read-only

The rule reads one `README.md` up to a **64 KiB** ceiling and takes one
bounded, symlink-refusing directory census. It executes no project
command, invokes no package manager, makes no network request, and
writes nothing: a recursive tree hash of every repository it examined is
identical before and after. A `README.md` above the ceiling is truncated
and the truncation is reported as a coverage gap, so a claim in the tail
is untested rather than verified.

### The measured corpus

Measured on the current tree. `design.md` §5 recorded 7 contradicted
repositories and 19 contradicted surfaces on 2026-10-03; six of those
seven READMEs have since been corrected, so the corpus is smaller now.

| Repository | README claim | Surface | Evidence | Outcome |
|---|---|---|---|---|
| `argoset` | `README.md:31` "live source adapters … not implemented in this checkout" | `application-source` | `apps/api/src/auth.rs` | clean — false positive, see below |
| `care-bridge` | `README.md:17` "No deployment is provisioned" | `deployment` | `compose.yaml` | clean — false positive, see below |
| `retake-kit` | `README.md:42` "no production deployment exists" | `deployment` | `compose.yaml` | clean — false positive, see below |
| `devloom` | `README.md:9` "nothing is deployed" | `deployment` | `compose.yaml`, `Dockerfile` | clean — false positive, see below |
| `kairosa` | `README.md:9` five named surfaces, all absent | — | 39 source files, `tests/`, a CI workflow, none of it claimed | clean — the claim is scoped |
| `chronicleaf` | `README.md:21` ten named features | — | n/a | no finding; 1 coverage gap |
| `opendockify` | `README.md:52` "not implemented in the MVP" | — | n/a | no finding; 1 coverage gap |
| `platform-contracts` | `README.md:15` "The project is planning-only." | — | n/a | no finding; 1 coverage gap |
| `pluggify`, `testify`, `aesthra`, `reelio`, `shopifireman`, `rostera` | claims now describe the tree accurately | — | — | clean |

**Four measured false positives, since fixed in the check.** The rule's first
live run flagged four repositories whose READMEs were accurate. All four are
clean now; the repair was to the evidence rules, not to any README, because
editing an accurate document into a false one to satisfy a wrong rule is the
failure mode this check exists to catch.

1. **`deployment` read the tree before the declaration.** A repository
   declaring `deployment.deployable: false` has, by its own declaration, no
   deployment surface, but a local `compose.yaml` — present in `care-bridge`,
   `retake-kit` and `devloom` purely to run locally — was enough to contradict
   an accurate claim. The rule now consults `deployment.deployable` first and
   only falls through to the tree when the repository claims to be deployable,
   so a real deployment is still caught and only the local case is forgiven.
2. **`application-source` resolved through bare `source`.** The `argoset`
   sentence is about *live source adapters for named platforms* — provider
   language — and has nothing to do with application source. The synonyms now
   require explicit application-source or extension-application wording, so
   such a sentence becomes a coverage gap instead of a contradiction.

Neither fix disables detection. `rostera`'s original wording still produces its
`build` and `ci` contradictions, and a repository declaring `deployable: true`
while holding a Compose file is still a contradiction; both directions are
pinned by regression tests.

### What this check does not enforce

- It does not verify a **positive** claim. "Five of 53 packages are
  implemented" is not checked; only present-tense absence claims are in
  scope.
- It does not decide **which side** of a contradiction is wrong. The
  finding names the line and the evidence path; the repository owner
  decides. Nothing here rewrites a README.
- It does not change any project's `lifecycle`, `evidence_status`,
  `deployable` flag, promotion state, or capability adoption posture, and
  it never promotes a capability to verified. A sentence in a README is
  prose, not evidence.
- It does not verify build, test, or release **success**. "Build/test
  success is not runtime or release evidence" holds unchanged.
- It does not run a project command, so a surface it cannot see from the
  tree is reported as untested, not as absent.
- It is not bound as a `driftwatchdog` gate check; that binding is owned
  by the `driftwatchdog` repository and is deferred.

## Default exclusions

The checker ignores the following directory segments everywhere inside
a product root, so projects do not need to repeat them:

`tests`, `test`, `__tests__`, `spec`, `specs`, `node_modules`, `vendor`,
`target`, `build`, `dist`, `out`, `.git`, `fixtures`, `fixture`,
`example`, `examples`, `generated`, `.github`, `.gitlab`.

The default list is restricted to universally non-product paths
(test, fixture, generated, vendor, build). Projects that store code in
`scripts/`, `docs/`, `deployment/`, `schemas/`, or `templates/` opt in
to scanning them by listing them in `product_roots` and any non-product
subtree in `exclude`.

## Language rules

- **Rust** — `#[test]` or `mod tests` outside a `#[cfg(test)]` guard.
  `#[cfg(any(test, ...))]` and `#[cfg(all(test, ...))]` do not authorise
  test scope. Repository `tests/` is always outside product scope.
- **Python** — `import unittest`/`from unittest`/`import pytest`/`from
  pytest` imports, or `def test_*` definitions in non-`test_*.py` files.
- **TypeScript / JavaScript** — `describe(`, `it(`, or `test(` calls in
  non-`.test.*` / non-`.spec.*` files. Files matching the test glob are
  excluded by path policy.
- **C#** — `[Test]`, `[TestMethod]`, `[Fact]`, `[Theory]` attributes.
- **Go** — `*_test.go` files (excluded by path policy).
- **Generic** — file-name heuristic: `test_*`, `*_test`, `*.test.*`,
  `*.spec.*` filenames anywhere in product scope.

## Ownership

| Surface                                  | Owner                                                |
|------------------------------------------|------------------------------------------------------|
| Checker, vocabulary, exit codes, contract| Workspace Governance (this repository).              |
| Quality policy and exclusions            | Each product repository via `.project.json` quality. |
| `product_roots`, `languages`, threshold  | Project owner, declared locally.                     |
| Gate execution                           | Driftwatchdog, via the existing Gate binding.        |
| Audit translation to `Finding`           | Workspace Governance audit (`workspace_check.py`).   |

## Known limits

**The Rust scan reads the module tree, but only one directory deep.** A `.rs`
file is treated as test scope when the crate declares its module under
`#[cfg(test)]`: a sibling `mod.rs`, `lib.rs` or `main.rs`, or the parent
directory's `mod.rs` for a directory module, carrying the guard on the
`mod <name>;` declaration that reaches the file — including through a
`#[path = "..."]` attribute, which is the shape
`rust-ecommerce/src/shipment/mod.rs` uses. A file with no such declaration is
product code even when it is called `tests.rs`, because a name is a convention
and an exclusion would hide a real finding rather than report one. The guard is
tracked per item, so `#[cfg(test)]`, `#[path = …]` and `mod tests;` on
consecutive lines are one guarded item and not three. An ambiguous guard
(`cfg(any(test, …))`, `cfg(all(test, …))`) authorizes nothing, as before.

What the reader cannot have: a `#[cfg(test)]` declaration reached through
`include!`, a build script, or a `#[path]` pointing outside the directory that
declares it, and inline modules such as `mod tests { … }` in a file that is
itself declared `#[cfg(test)]` elsewhere — those are still scanned and still
reported. The languages other than Rust are unchanged and keep their
file-name-based heuristics.

**A `tests/` directory inside a product root is out of scope before the module
tree is consulted.** `DEFAULT_EXCLUDED_DIRS` matches the path *segment* `tests`,
so `src/gate/governance/tests/mod.rs` is never scanned whether or not a parent
`mod.rs` declares it. That is the intended shape — test roots are first-class —
and it is why the fix needed no `tests`-directory handling.

## Non-goals

- No automatic code repair, test generation, or AI judgment.
- No claim that a passing lexical check proves behaviour, integration,
  release, or runtime quality.
- No universal ban on tests in a repository; test roots remain first-class.
- No hard-coded assumption that every project uses Rust, Python, .NET,
  or JavaScript; the `generic` profile covers everything else.
- No replacement for Driftwatchdog's Gate executor; the checker is one
  of the checks Driftwatchdog binds.
