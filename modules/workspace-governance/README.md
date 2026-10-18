# Workspace Governance

Workspace Governance coordinates a portable workspace selected at invocation
time. It owns the project registry, governance profiles, bootstrap templates,
evidence vocabulary, and deployment declarations.

It does not own product source code, language runtimes, shared gate runtimes, or production execution.

## Reusable runtime templates

`templates/runtime` contains versioned Dockerfile and Compose families for
common web, static-site, and worker shapes. Use `scripts/runtime_templates.py`
to inspect a project, render an explicitly selected family, and check the
generated contract. Rendering records configuration evidence only; it never
promotes `deployment.deployable` or embeds secrets. See
`docs/deployment-templates.md`.

## Ownership

- `forge` is the portfolio control-plane consumer.
- `driftwatchdog` is the shared quality and evidence gate runtime.
- `sisyphusfy` is the agent iteration runtime.
- `ariadex` is the supervised runtime operator.
- `dotnet-platform-libs` is the reusable .NET library platform.
- `jenkins-local` performs Jenkins/Mac deployment execution.

## Capability and release evidence

Projects may add two optional blocks to `.project.json`:

- `capabilities` — declares reusable platform capabilities (identity, admin,
  tenancy, billing, jobs, storage, release, and project-defined names) with
  an owner and an evidence state, plus an optional additive `consumes[]` array
  recording which registered provider package this project pins.
  Declarations are intent, not implementation evidence. See
  `docs/capabilities.md` for the canonical vocabulary.
- `release_evidence` — a producer-neutral record (revision, version,
  toolchain, artifacts, digests, SBOM, provenance, checks, publication) that
  backs a deployment. A `deployable: true` project must carry a complete
  `release_evidence` block; otherwise `workspace_check.py` fails the project
  closed with `RELEASE_EVIDENCE_INSUFFICIENT` and `jenkins_manifest.py` will
  not emit the entry.

Both blocks are additive. Existing declarations without them remain valid;
the audit only surfaces findings for projects that opt in. The new fields do
not, and never will, promote a project to deployable on their own.

## Capability adoption gate

Whether a project actually consumes a registered shared capability is
**measured, not declared**. `capabilities.json` binds each capability id to
exactly one provider and assigns a posture; the audit joins that registry with
the consumer's own manifests and the consumer's `consumes[]` record, and all
three must agree.

- a repository cannot mark itself compliant — a hand-written `consumes[]`
  entry with no resolvable manifest edge is a finding;
- findings escalate from WARNING to ERROR only on a **recorded** graduation
  (verified provider + consecutive audit count + an appended ledger entry),
  never on elapsed time;
- legitimate non-adoption stays possible through an evidence-backed
  `declines[]` record with a closed rationale vocabulary.

The measurement is read-only: no project command is executed, no package
manager is invoked, no lockfile is resolved, and no network request is made.
Six new finding codes (`CAPABILITY_ADOPTION_MISSING`, `_UNEVIDENCED`,
`_EVIDENCE_STALE`, `_DECLINE_UNRECORDED`, `_UNRESOLVED`, plus the governance
`CAPABILITY_REGISTRY_INVALID`) are additive; every existing finding code,
severity, exit code, report key, and Forge adapter observation key is unchanged.
See `docs/capabilities.md` and `docs/adoption-grant.md`.

## Product-code quality gate

Projects may opt in to the product-code quality gate by adding a `quality`
block to `.project.json` and binding `python3 scripts/product_code_quality.py
<dir>` in their `.ai-gate/gate.yaml`. The gate detects two failure modes
the structural audit cannot: test code in product scope (including lexical
Rust `#[cfg(test)]` containment) and placeholder debt across product files
(TODO, FIXME, XXX, HACK, placeholder, stub, unimplemented!, todo!). The
checker is fail-closed: missing or invalid policy, unsupported language,
traversal in exclusions, or an unreadable product file is REVIEW_REQUIRED,
never a guessed pass. See `docs/product-code-quality.md` for the full
contract, JSON schema, and per-language rules.

One rule in that checker is not opt-in. A README status line that denies
implementation in the present tense is verified against the file tree, on
the closed set of surfaces that line names and on nothing else — so a
repository with a test suite and an accurate status line stays clean. A
contradicted surface is ERROR `QUALITY_README_CLAIM_CONTRADICTED`; a
status statement that names no decidable surface is counted once per run
as the fleet-level `QUALITY_README_CLAIM_UNSCOPED` coverage metric, never
as a finding against the repository that wrote it. There is no
`readme_claims` block, no `mode`, and no exemption, because a check that
can be switched off is not a check. See the "README status claims" section
of `docs/product-code-quality.md`.

## Gate command binding

A check the gate selects but has no command for is a check that never runs,
and the gate counts it as `UNBOUND` and fails the project. `scripts/bind_gate_commands.py`
derives those commands from what is on disk in each repository and writes them
into `.ai-gate/gate.yaml`.

```bash
# Dry run: classify every check, write nothing.
python3 scripts/bind_gate_commands.py --root /home/paul/code

# Write the bindings and refresh the debt record.
python3 scripts/bind_gate_commands.py --root /home/paul/code --apply

# Refresh only the debt record; the manifest run is forced dry.
python3 scripts/bind_gate_commands.py --root /home/paul/code --debt-only
```

The default run is read-only and exits `3` while declared checks remain
unresolved; it exits `0` when they are all bound and `2` when the workspace
root or invocation is unusable. `--repo <name>` limits a run to named
repositories, and `--verbose` prints each derived command with its evidence.
The dry run still reports debt so owners can review it before `--apply`.

The default run is read-only and exits `3` while declared checks remain
unresolved; it exits `0` when they are all bound and `2` when the workspace
root or invocation is unusable. `--repo <name>` limits a run to named
repositories, and `--verbose` prints each derived command with its evidence.
The dry-run still reports debt so owners can review it before `--apply`.

### It refuses rather than guesses

The tool's rule is *a wrong command is worse than a listed gap*. A guessed
command is worse because it is invisible; a listed gap is on the record, names
its own cause, and is read by the next person. So it binds one command, or
refuses and says why.

Three outcomes, in order:

| Situation | Result |
|---|---|
| One candidate | Bound |
| A root-level entry point beats a nested one | The root is bound |
| Candidates spanning two or more stacks | Joined with ` && ` |
| Two candidates of the same stack | **Refused**, naming both |

The composite case is the one worth understanding. A repository with a .NET
backend and an npm frontend needs both built, so the tool joins them rather
than picking one:

```
(dotnet build backend/App.slnx) && (cd frontend && npm run build)
```

Each part is wrapped in a subshell, and that is load-bearing. A candidate may
`cd` into its own directory, and `cd` is not scoped to one command in an `&&`
chain — joined flat, the second `cd frontend` would run from inside the first
part's directory, where it does not exist.

Two candidates of the **same** stack still refuse. Two Flutter packages are
one product described twice; composing them would run the same suite twice and
picking one would skip the other.

### The debt record

Every refusal is written to `gate-debt.json` in this repository, grouped by
cause, with the specific reason per check. It is the durable statement of what
is still unbound.

`GateOracleTest` pins that record **set-equal** to what the real gate reports
as unbound, by running the gate itself. If the tool ever stops finding a check
the gate still fails on, that test goes red rather than the record quietly
drifting. `--debt-out` redirects the record; the default follows `--root`.

### A new change waits on the debt

A repository listed in `gate-debt.json` may not open a new OpenSpec change.
`workspace_check.py` fails it with `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` and
names the unbound checks, so debt is paid as work starts rather than in a
cleanup that never happens.

Changes that were already open when the rule landed are listed in
`gate-debt-baseline.json` and stay silent; anything opened afterwards is
reported. `close-unbound-gate-checks` is never a trigger — a repository must
be able to start the work that fixes its debt. A deliberate exception is a
committed line in `gate-debt-exceptions.json` with its reason, and it is
reported at WARNING so it stays visible. All three records fail closed: a
missing or malformed one is an error, never read as "no debt".

This repository permits only one active OpenSpec change at a time.
`workspace_check.py` reports an ERROR when multiple change directories are
active or when `HANDOFF.md`'s single `current_spec` pointer does not match the
active directory (`OPENSPEC_CHANGE_SERIALITY`). A repository-state assertion
in the required unit-test Gate blocks this repository in either case. Finish
and archive the current change before creating another; set `current_spec:
none` when no change is active. The seriality finding is scoped to this
repository until the shared Gate runtime supports its finding code.

## Registry and declaration integrity

`projects.json` is the single source of project identity, and the audit fails
closed on:

- a declaration whose `id` differs from its registry `id` (`DECLARATION_ID`);
- a declaration missing `schema_version: 1` or the required `verification` and
  `deployment` blocks (`DECLARATION_SCHEMA`);
- a real command supplied under a non-schema key (`VERIFICATION_COMMAND`);
- an adopted registry entry with no `.project.json` (`DECLARATION_MISSING`);
- a discovered declaration with no registry entry (`DISCOVERED_UNREGISTERED`);
- two registry entries sharing an `id` (`PROJECT_DUPLICATE`) or a `path`
  (`PROJECT_PATH_DUPLICATE`).

The canonical `id` is derived from the directory name (`hestiaLab` →
`hestia-lab`). `init_project.py` refuses an explicit `--id` that conflicts with
the derived id; a directory name that is not convertible (for example a dotted
host name) may use an explicitly recorded alias.

`scripts/maturity_audit.py` reports the error and warning counts produced by
`workspace_check.py`; it never computes a second, independent result, so the
two views cannot disagree.

## Toolchain decision register

`decisions.json` at the governance root is the single machine-readable
source of accepted workspace toolchain decisions. Inspect it with
`python3 scripts/workspace_check.py --decisions` (or `--json`); the
canonical prose view is `docs/toolchain-decisions.md`.

## Portfolio vocabulary

`vocabulary.json` is the single machine-readable source for the canonical
`profiles` (15), canonical `kinds` (12), the two classification axes
`roles` (6) and `stacks` (6), `placeholder_markers`, and
`secret_field_substrings`. `workspace_check.py` enforces the canonical sets
(a non-canonical registry `profile` is ERROR `PROFILE_UNKNOWN`; a
declaration `kind` that is neither a member of the legacy `kinds` list nor the
value its own canonical `role`/`stack` derive is WARNING `KIND_UNKNOWN`; a
`profiles/<name>.json` document naming a non-canonical profile is ERROR
`PROFILE_DOC_UNKNOWN`) and scans secrets with the vocabulary substrings, while
`product_code_quality.py` defaults its placeholder markers from the same file.
A missing, unparseable, duplicate-bearing, or empty-list vocabulary makes each
consumer exit non-zero; there is no empty-list fallback.

`placeholder_markers` holds only unambiguous code-debt tokens: `todo`,
`fixme`, `xxx`, `hack`, `unimplemented!`, `todo!`. An ordinary English word is
not a marker, because the scan matches case-insensitively on word boundaries
over every product line — prose describing the word, a doc comment naming it,
and the gate's own `placeholder-threshold` identifier all match it. Measured
over `driftwatchdog/src/`, `placeholder` matched 26 times and none of the 26
was unfinished work. A per-project `placeholder_markers` override is still
honoured, and is left exactly as its author wrote it.
 Kind is descriptive
and never drives profile requirements. The canonical spec is
`openspec/specs/portfolio-vocabulary/spec.md`.

Projects use independent `role` and `stack` axes; `kind` remains their derived,
deprecated composite. The audit checks the axes and registry agreement without
changing project requirements. See [the vocabulary guide](docs/vocabulary.md).

Archiving a change writes a marker into the `## Purpose` of every spec it
produced. ERROR `SPEC_PURPOSE_PLACEHOLDER` is **not enabled** until the backfill
is applied, and `scripts/spec_purpose_backfill.py` is the reviewable tool that
applies it. See `docs/vocabulary.md`.

## Vocabulary

`docs/vocabulary.md` is the one entry point for the registry and
evidence terms: declaration (intent), `evidence_status`,
`capabilities`, `release_evidence`, `deployable`, the promotion
ledger, finding codes, `kind`/`profile`, the quality gate, the
governance front door, and the Forge adapter observation. Each
definition links its canonical source instead of restating policy.

The invariant across all of them: **a declaration is intent, not
evidence; `deployable` requires both a promotion-ledger entry and
complete release evidence.** Registry presence never proves that a
project builds, passes tests, deploys, or is runtime-healthy.

## Mandatory command

Run from this repository. The registry never stores a machine-specific root;
pass a workspace root relative to the current checkout, or an absolute path
only at invocation time:

```bash
python3 scripts/workspace_check.py --self-check
python3 scripts/workspace_check.py --root .. --audit --json
python3 scripts/auto_gate.py --root .. --report auto-gate-report.json
python3 scripts/product_code_quality.py <project_dir> --json
python3 -m unittest discover -s tests -v
```

## Usage

One copy-paste example per governance command. Every example uses
only flags the shipped scripts support (each runs `--help`
successfully except the Forge adapter, which reads one JSON
request on stdin). Pass a workspace root relative to the current
checkout, never a machine-specific root.

```bash
# Self-check: this repository's own registry and declarations (0 errors, 0 warnings)
python3 scripts/workspace_check.py --self-check

# Portfolio audit: every registered project (currently 0 errors, 16 warnings:
# 9 HANDOFF_SCAFFOLD, 7 DEPLOYMENT_PROMOTION_STALE); add --json for the machine-readable report
python3 scripts/workspace_check.py --root .. --audit --json

# Single project audit
python3 scripts/workspace_check.py --root .. --project my-project

# Unattended gate (fails on governance or deployment errors; writes a report when requested)
python3 scripts/auto_gate.py --root .. --report auto-gate-report.json

# Product-code quality gate for one project directory (exit 0 PASS, 1 FAIL, 2 REVIEW_REQUIRED)
python3 scripts/product_code_quality.py <project_dir> --json

# Portfolio maturity report (evidence states, CI coverage, OpenSpec adoption)
python3 scripts/maturity_audit.py --root .. --json

# Shared-rules drift triage (report-only; names projects missing a canonical .ai-rules clause, never edits)
python3 scripts/rules_drift.py --root .. --json

# Skill reference check (every path cited by the governance front-door skill must exist)
python3 scripts/check_skill_refs.py
```

```bash
# Onboard a project (non-destructive; writes only missing declarations, then review and fill the real verification command)
python3 scripts/init_project.py /path/to/project --profile rust-product --kind product
python3 scripts/workspace_check.py --root .. --project my-project
```

The initializer validates the profile against the gate binder and runs a
repository-scoped bind after creating the manifest. Commands are recorded only
when repository evidence supports them. An empty scaffold can therefore finish
with unresolved checks; run the workspace binder again after adding the real
build and test entry points.

```bash
# Promote / demote (the only path to deployable: true; records the evidence reference, never runs verification itself)
python3 scripts/promote_project.py <project-id> --evidence-ref "<revision-or-build pointer>"
python3 scripts/promote_project.py <project-id> --demote
```

```bash
# Deployment handoff (after the governance check; with --root .. currently 7 projects, 0 errors)
python3 scripts/jenkins_manifest.py --check --root ..
```

```bash
# Forge governance-provider adapter (read-only; one check request on stdin, one observation document on stdout)
echo '{"contract":"0.1.0","action":"check","project_id":"<id>","project_path":"<path>"}' \
  | WORKSPACE_ROOT=<workspace-root> \
    python3 scripts/forge_governance_adapter.py
```

## Terminal capture

`docs/assets/terminal-self-check.txt` is a checked-in, redacted
capture of a real `--self-check` (0 errors, 0 warnings) and
`--audit --json` (0 errors, 16 warnings with the finding
breakdown) run, plus one clearly-labelled illustrative finding
showing the shape of an unhealthy entry. It is reproducible
offline with the two commands at the top of that file.

Screenshots are not applicable: this project is CLI-only and has
no GUI surface, so terminal captures are the only visual
evidence.

### Optional: Forge governance-provider adapter

This repository ships a read-only shim that exposes the per-project audit
through Forge's published v0.1.0 governance-provider contract. The
adapter is not a CI surface — it only answers a single `check` request
on stdin and prints one observation document on stdout:

```bash
echo '{"contract":"0.1.0","action":"check","project_id":"<id>","project_path":"<path>"}' \
  | WORKSPACE_ROOT=<workspace-root> \
    python3 scripts/forge_governance_adapter.py
```

The exit code is `0` for every answerable evaluation (including `fail`,
`blocked`, and `unknown`), `2` for a malformed request, and `3` for an
audit crash (which Forge classifies as `unavailable`). The adapter never
writes, registers, adopts, or mutates anything in this repository, and
redacts secret-shaped substrings from every evidence line. Point Forge
at the script with:

```text
forge governance use workspace-governance <workspace-root> \
  --adapter <repo>/scripts/forge_governance_adapter.py
```

See `openspec/specs/forge-governance-adapter/spec.md` for the contract
shape and `tests/test_forge_governance_adapter.py` for the mapping
matrix.

### Agent skill: workspace-governance

`skills/workspace-governance/SKILL.md` is the single auto-triggering entry
for governance situations. It routes to per-concern references
(`skills/workspace-governance/references/*.md`), each linking the owning
script or canonical spec rather than restating policy. The skill is
versioned with the scripts it cites and is not installed automatically;
make it discoverable with a symlink or a copy:

```bash
ln -s <repo>/skills/workspace-governance ~/.agents/skills/workspace-governance
```

`python3 scripts/check_skill_refs.py` proves every cited path exists; it
fails naming the reference and the missing path when one does not.

With `explicit-plus-immediate-directories`, immediate child folders containing
common project markers are discovered automatically. The registry's
`discovery.unregistered_policy` decides the level: this workspace sets `fail`,
so an unregistered declaration is `DISCOVERED_UNREGISTERED` **ERROR** and must
be registered (or explicitly classified) before the audit passes. A discovered
folder is never treated as adopted or deployable until its local metadata and
evidence pass.

`auto_gate.py` is the unattended entry point. It fails on governance or
deployment errors, writes a machine-readable report when requested, and is
scheduled by the repository workflow. Jenkins should run the same command
before project verification or deployment; no manual audit request is needed.

## Onboard a project

```bash
python3 scripts/init_project.py /path/to/project \
  --profile rust-product --kind product
python3 scripts/workspace_check.py --root .. --project my-project
```

## Standalone project boundary

A project may declare a `standalone` block and an `integrations` array in
its `.project.json` to describe how it operates without sibling checkouts
or private services. The audit classifies each adopted project as one
of six canonical statuses — `standalone`,
`standalone-with-optional-integrations`, `library-dependent`,
`workspace-dependent`, `service-dependent`, or `blocked` — and reports
findings for absolute paths, `..` escapes, missing evidence, optional
provider gaps, and secret-shaped values. The Forge governance adapter
exposes the same classification as four additive observation fields
(`standalone_status`, `standalone_observed_revision`,
`standalone_integrations`, `standalone_findings`); existing keys and
exit codes are unchanged. See
`openspec/specs/standalone-project-boundary/spec.md` for the full
contract and finding codes.

## License and GitHub discoverability

The governance audit exposes standard projects that lack a root `LICENSE`,
source-backed `.project.json.discoverability` metadata, or verified GitHub
publication evidence:

```bash
python3 scripts/workspace_check.py --root .. --audit --json
```

Repair local metadata explicitly; semantic fields are never invented and
existing files are preserved:

```bash
python3 scripts/init_project.py /path/to/project --profile product --kind product \
  --apply-metadata --description "Source-backed description" \
  --keywords repository governance --topics repository-governance \
  --audience developers --use-case "audit project repositories" \
  --search-phrase "repository governance tool" \
  --homepage https://github.com/OWNER/REPOSITORY --license-spdx MIT \
  --visibility public
```

Verify or explicitly apply GitHub metadata through the authenticated `gh` CLI:

```bash
python3 scripts/github_metadata.py /path/to/project --check --json
python3 scripts/github_metadata.py /path/to/project --apply --json
python3 scripts/github_metadata.py /path/to/project --publish \
  --owner OWNER --repository REPOSITORY --json
```

Authentication failures are blocking; run `gh auth login` manually. The
adapter never handles or prints tokens, never replaces an existing remote, and
never force-pushes. GitHub metadata is evidence only after `gh repo view` and
the local publication declaration agree.

`--id` is optional and defaults to the kebab-case directory name; an explicit
`--id` must match the derived id unless the directory name is not convertible.

The initializer is non-destructive and writes only missing governance declarations and rules.
It never copies gate runtimes, installers, credentials, or universal rule implementations into a product repository.

## Deployment promotion

A project becomes `deployable: true` only through a recorded promotion
transaction. The script `scripts/promote_project.py` runs the project
audit, verifies the compose file, requires an explicit evidence reference,
mutates the `deployment.*` block in `.project.json`, and appends a
tamper-evident entry to `deployment/promotions.json`. Hand-edits that set
`deployable: true` without a matching ledger entry fail the audit with
`DEPLOYMENT_PROMOTION`; a stale revision (the project's HEAD moved after
promotion) raises the triage-only `DEPLOYMENT_PROMOTION_STALE` warning.

```bash
python3 scripts/promote_project.py <project-id> \
  --evidence-ref "<revision-or-build pointer>" \
  [--compose-file <name>] [--jenkins-job <name>] [--actor <name>] [--force]

python3 scripts/promote_project.py <project-id> --demote
```

The script never runs a project verification itself; it records the
evidence reference the operator supplies, so the audit boundary stays
with the project's own verification pipeline.

## Jenkins

`deployment/projects.json` is the safe handoff to `jenkins-local`. Generate it only after the governance check:

```bash
python3 scripts/jenkins_manifest.py --check
```

The manifest enumerates only projects with a current `deployable: true`
declaration backed by a promotion ledger entry. Jenkins still owns
synchronization, secrets, Compose, database provisioning, job registration,
and deployment. The governance project only says which projects are
eligible and which verification command must pass.

## Status boundary

Registry presence is not implementation evidence. The checker records declarations and policy status; project gates must provide build, test, integration, provider, runtime, and release evidence separately.
