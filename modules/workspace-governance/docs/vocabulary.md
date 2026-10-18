# Registry and Evidence Vocabulary

One entry point for the registry/evidence terms used across this
repository. Each definition links its canonical source rather than
restating policy; the sources own the wording.

**Invariant: a declaration is intent, not evidence.** Registry
presence and a `.project.json` declaration record what a project
claims. Build, test, integration, provider, runtime, and release
health are proven only by the project's own verification command
and runtime observation. Nothing in this repository promotes a
project to deployable on metadata alone.

| Term | Meaning | Canonical source |
|---|---|---|
| Declaration (`.project.json`) | Intent: what a project claims (identity, verification command, deployment flags, capabilities, evidence blocks). | `schemas/project.schema.json`, `openspec/specs/registry-integrity/spec.md` |
| `evidence_status` | Verification state of the declaration: `declared`, `configured`, `verified`, `unverified` (plus the `planned` value still found in un-remediated siblings). | `docs/capabilities.md` |
| `capabilities` | Declared reusable platform capabilities (`identity`, `admin`, `tenancy`, `billing`, `jobs`, `storage`, `release`, and project-defined names) with an owner and an evidence state. | `docs/capabilities.md` |
| Capability adoption | Whether a project consumes a registered shared capability, **measured** by joining `capabilities.json` with the consumer's manifests and its `consumes[]` record. Not a self-declared state. | `docs/capabilities.md`, `capabilities.json` |
| `declines` | Evidence-backed non-adoption record; the lawful exit from a capability adoption finding other than adoption, with a closed rationale vocabulary. | `docs/capabilities.md` |
| `adoption_grant` | Bounded, single-use, expiring permission to repoint this repository onto one registered provider for one capability, without cross-repo authorization. The provider root is always read-only. | `docs/adoption-grant.md` |
| Adoption ledger | Append-only, chained record of executed adoptions in `adoption/adoptions.json`. Adoption is not promotion. | `docs/adoption-grant.md` |
| `release_evidence` | Producer-neutral record (revision, version, toolchain, artifacts, digests, SBOM, provenance, checks, publication) backing a deployment. | `docs/capabilities.md` |
| `deployable: true` | Requires **both** a matching promotion-ledger entry **and** a complete `release_evidence` block; neither alone suffices. | `openspec/specs/deployable-batch-onboarding/spec.md`, `docs/deployment-promotion.md` |
| Promotion ledger | Tamper-evident record of promotion decisions in `deployment/promotions.json` (with `deployment/promotions.sha256` sidecar). | `docs/deployment-promotion.md` |
| Finding code | A stable audit result name with a level (`ERROR`/`WARNING`), e.g. `DECLARATION_ID`, `KIND_UNKNOWN`, `HANDOFF_SCAFFOLD`, `DEPLOYMENT_PROMOTION_STALE`. | `openspec/specs/registry-integrity/spec.md` |
| `kind` / `profile` | Descriptive project classification. `profile` is drawn from the 15 canonical profiles; `kind` is the derived, deprecated composite of `role` and `stack`, so its authority is the two axes rather than the frozen legacy 12-value `kinds` list. | `vocabulary.json`, `openspec/specs/portfolio-vocabulary/spec.md` |
| `role` / `stack` | The two independent classification axes: what a project is **for**, and what it is written **in**. `stack` is single-valued; there is no `stacks` array. | `vocabulary.json` `roles`/`stacks`, `openspec/specs/identity-and-taxonomy/spec.md` |
| Identity (`id` / `path` / `alias`) | `id` is the immutable join key and is never derived from or renamed to match `path`. `path` is a registry-only resolution detail. A directory whose name is not a valid identifier records an explicit `alias`; the eight `campusMesh`-style entries are already correct and must not be churned. | `openspec/specs/registry-integrity/spec.md`, `scripts/init_project.py` |
| Quality gate (`quality` block) | Opt-in product-code rule (test code in product scope, placeholder debt); fail-closed, never a guessed pass. | `docs/product-code-quality.md`, `openspec/specs/product-code-quality-boundary/spec.md` |
| Governance front door | The single auto-triggering skill entry routing each governance situation to its per-concern reference. | `skills/workspace-governance/SKILL.md`, `openspec/specs/governance-front-door/spec.md` |
| Forge adapter observation | Read-only translation of this repository's audit findings for Forge's governance-provider contract; not runtime or release evidence. The v0.1.0 contract now also carries additive `standalone_status`, `standalone_observed_revision`, `standalone_integrations`, and `standalone_findings` fields. | `openspec/specs/forge-governance-adapter/spec.md`, `openspec/specs/standalone-project-boundary/spec.md` |
| Standalone classification (`standalone` / `integrations`) | Optional `standalone` block and `integrations` array in `.project.json` record how a project operates without sibling checkouts or private services. The audit classifies the project as one of six canonical statuses (`standalone`, `standalone-with-optional-integrations`, `library-dependent`, `workspace-dependent`, `service-dependent`, `blocked`) without executing project commands. Declarations are intent, not evidence; an absent block reports `blocked` + `STANDALONE_BLOCKED` warning, never a pass. | `openspec/specs/standalone-project-boundary/spec.md` |

## The two classification axes

`kind` answered two questions in one string, which is why a repository written
in an interesting language could not state its role. The two questions are now
separate fields, and every combination is legal — including
`role: platform, stack: rust`.

| Axis | Canonical values (`vocabulary.json`) | Question it answers |
|---|---|---|
| `role` | `product`, `platform`, `tooling`, `governance`, `deployment`, `inventory` | What is this repository for? `shared-library` is **not** a role: by owner ruling the three repositories that wrote it take `role: platform`, because the library-versus-service distinction that carries weight already lives in the frozen `profile` values and no consumer branches on the role. |
| `stack` | `dotnet`, `rust`, `typescript`, `python`, `flutter`, `none` | What is it written in? Single-valued by owner ruling; `lexora`, `opendockify` and `trippify` all resolve to `stack: dotnet` even though they also ship a Flutter client. |

Both sets are declared once in `vocabulary.json` and mirrored as
`$defs/taxonomy_role` / `$defs/taxonomy_stack` in
`schemas/project.schema.json`. Neither axis is in the schema's `required` list:
a declaration that omits them stays schema-valid and reports
`TAXONOMY_AXIS_UNKNOWN`. The axes are **descriptive** — they never drive a
required file, a CI requirement, a verification command, or any evidence,
deployment, or promotion state. `profile_requirements` reads the registry
`profile` only and is unchanged.

### `kind` is derived, never authored

`kind` stays in the schema's `required` list and stays in all declarations, as
the derived composite `<stack>-<role>`, or the bare `<role>` when the stack is
`none`. Keeping it costs one function and stops every undocumented consumer of
the old string from breaking. It is never written by hand and never newly
required; the migration does not rewrite it, so a hand-authored value that
disagrees with its own axes is still visible as `TAXONOMY_LEGACY_VALUE`.

### The derived form is the rule; `kinds` is a legacy vocabulary

`kinds` in `vocabulary.json` is the original 12 hand-authored values. It is a
**legacy vocabulary, not the authority on a derived value**, and it is
deliberately left frozen at 12.

A `kind` is canonical when it is either

1. a member of the legacy `kinds` list, or
2. the value its own two axes derive — `kind == derive_kind(role, stack)` with
   both axes present and canonical.

That second clause exists because the derivation is a **product of two
enumerated axis sets**: 6 roles × 6 stacks is 36 values, of which **26 are not
in `kinds`** (the 12 listed values overlap the derivation on only 10 — `shared-library`
and `typescript-monorepo` are pre-axis shapes, not role/stack pairs). A `kind`
that is correct by the current rule is therefore often
absent from the list, and holding a derived value to the list reports a defect
that no owner can act on. The 17 findings that motivated this
(`rust-platform` on `forge`, `rust-tooling` on `driftwatchdog`,
`python-governance` on this repository, and 14 others) were all of that shape.

Extending `kinds` to cover the derivation would relocate the problem rather
than solve it: the list would need re-extending for every future axis
combination, which is the same unbounded manual-maintenance point found in
`driftwatchdog`'s `GOVERNANCE_QUALITY_DECLARED_CODES` allow-list. It would
also freeze a derived field's value space into a second, contradictory source
of truth. So `kinds` stays 12, and the axes are the authority.

The rule is symmetric in what it refuses. All of the following still report
`KIND_UNKNOWN` at WARNING, because none of them is a self-consistent
derivation:

| Declaration | Reports | Why |
|---|---|---|
| `role: platform, stack: rust, kind: rust-governance` | `KIND_UNKNOWN` + `TAXONOMY_LEGACY_VALUE` | a hand-authored composite that disagrees with its own axes |
| `role: tooling, stack: rust, kind: quality-gate` | `KIND_UNKNOWN` | in neither the legacy list nor the derived form |
| `kind: rust-platform` with no axes | `KIND_UNKNOWN` | nothing has been derived, so the value is not measurable |
| `role: library, stack: rust, kind: rust-platform` | `KIND_UNKNOWN` + `TAXONOMY_AXIS_UNKNOWN` | the axes are non-canonical, so there is no derivation to compare against |
| `kind` absent or not a string | `KIND_UNKNOWN` | unchanged |

`KIND_UNKNOWN` stays WARNING and is not repurposed, narrowed away, or
silenced: a value that is neither legacy nor derived is still a finding, and
`validate_kind` is the same single place that reports it.

### Finding codes

| Code | Level | Fires when |
|---|---|---|
| `TAXONOMY_AXIS_UNKNOWN` | WARNING | `role` or `stack` is absent or outside its canonical set; the message names the offending value and both sets. An axis present on only one of the two surfaces lands here too, because an absent value is not a contradiction. |
| `TAXONOMY_MISMATCH` | ERROR | The registry entry and the declaration disagree on `role` or `stack`; the message names both values and the project id. |
| `TAXONOMY_LEGACY_VALUE` | WARNING | A `kind` disagrees with the value derived from its own axes, including a stack-qualified composite still present while both axes are declared. |

`KIND_UNKNOWN` keeps its shipped meaning and its WARNING level (a `kind` that
is neither in the legacy 12-value list nor the value its own axes derive) and
is not repurposed.

### README-claim vocabulary

`vocabulary.json` also carries two additive arrays for the
`readme-claim-verification` capability. Both are non-empty and duplicate-free,
and neither narrows an existing array.

| Key | Contents | Question it answers |
|---|---|---|
| `readme_claim_surfaces` | the 13 canonical surface keys: `api`, `application-source`, `build`, `ci`, `deployment`, `migrations`, `mobile-client`, `production-evidence`, `provider-integration`, `repository-content`, `runtime`, `test`, `web-client` | Which surfaces can this check decide from a file tree? A surface with no evidence rule is not a check: `data-set` was dropped by owner ruling for exactly that reason, so `kairosa`'s `data set` phrase is knowingly **unchecked** rather than verified. |
| `readme_absence_phrases` | the present-tense absence triggers, including the bounded wildcard form `no ... exists` that lets one sentence name a list of surfaces | Which sentences are claims about the repository's current state? |

The loader is fail-closed for both: a missing, empty, or duplicate-bearing list
raises `VocabularyError` naming the file, and a surface key with no evidence rule
in `product_code_quality.py` is rejected too. An empty surface list would make
every claim vacuously true, which is a silent pass on the one thing the rule
exists to catch.

### README-claim finding codes

| Code | Level | Fires when |
|---|---|---|
| `QUALITY_README_CLAIM_CONTRADICTED` | ERROR | A surface named by a qualifying README claim is present in the file tree. One finding per contradicted surface, naming the README line(s), the surface, and the evidence path. |
| `QUALITY_README_CLAIM_UNSCOPED` | WARNING, emitted once per run | A negative status statement names no surface this check can resolve. A fleet-level CHECKER-COVERAGE metric, `REVIEW_REQUIRED` in the checker's vocabulary, naming every unscoped statement and its repository. Never a per-repository finding and never a pass: it fires on repositories whose claims are accurate. |

Both are separate from the `QUALITY_*` codes above and from the `TAXONOMY_*`
family; no existing code is renamed, narrowed, or reused. There is no
`readme_claims` declaration field — a declaration carrying one is reported as
`WARNING QUALITY_REVIEW_REQUIRED` naming the unsupported field, and the check
runs anyway. See `docs/product-code-quality.md`.

### Migration state — applied, and the check is enabled

`TAXONOMY_MISMATCH` is ERROR by owner ruling, and the whole taxonomy audit was
held off the fleet until the migration was applied. That ordering is now
spent: the flag `TAXONOMY_AUDIT_ENABLED` in `scripts/workspace_check.py` is
**True**, and the severity was never softened to WARNING to get there. The
sequencing was enforced by the checker itself, not by a comment and not by an
environment variable — with the flag on, the same unmigrated fixture that is
silent with the flag off reports ERROR `TAXONOMY_MISMATCH`.

The migration is a fleet-wide cross-repo source edit and was applied under
explicit authorization. It is repeatable and reviewable:

```bash
# 1. review the plan (dry run; writes nothing)
python3 scripts/axis_migration.py

# 2. has it been applied? exit 3 while any row is outstanding
python3 scripts/axis_migration.py --check

# 3. apply it, then set TAXONOMY_AUDIT_ENABLED = True in the same commit
python3 scripts/axis_migration.py --apply
```

As landed, step 2 exits 0 and step 3 is a no-op re-run: every declaration and
registry entry already carries the planned axes, and all 81 declarations
satisfy `kind == derive_kind(role, stack)`.

The committed plan is `migration/identity-and-taxonomy-axis-split.json`: one row
per declaration with the resolved `role`/`stack`, the rule that resolved it, and
the evidence behind it. The tool reads the decided rows out of the change's
`design.md` rather than keeping a second copy, and refuses to plan if the fleet
has drifted from that table or if any existing registry `id`/`path` pair would
move.

## The archived-change Purpose marker

Archiving a change writes one line into the `## Purpose` of every spec that
change produced:

```markdown
## Purpose
TBD - created by archiving change <name>. Update Purpose after archive.
```

The archiver is third-party (`@fission-ai/openspec`), so the defect is held here
instead: `scripts/spec_purpose_backfill.py` resolves each one mechanically and
`SPEC_PURPOSE_PLACEHOLDER` stops the next from appearing. A spec whose purpose is
the string `TBD` reads as a finished spec, and 1,234 of them did.

Two generations exist and are one defect: the current form with the trailing
`Update Purpose after archive.` sentence (1,229 files) and the earlier short form
without it (5 files). 7 of the 1,234 carry a blank line between the heading and
the marker. The change name is the **bare** name while the archive directory is
date-prefixed (`archive/2026-10-03-work-on-main-baseline-rule/`), so the two are
joined by stripping `YYYY-MM-DD-`, never by concatenation.

The marker must be the whole `## Purpose` **section** body, not merely a line
starting with it: a real sentence appended after the marker is a real purpose and
is never a finding. The change name is bound to `[\w.-]+` to make that hold, since
a change name has no spaces and trailing prose can never be absorbed into one.

### Finding codes

| Code | Level | Fires when |
|---|---|---|
| `SPEC_PURPOSE_PLACEHOLDER` | ERROR | A spec file's `## Purpose` section still holds the marker. The message names the spec path relative to its repository and the change the marker records, so the remediation is "resolve this spec from this change". A repository with no `openspec/specs` tree is skipped, and the rule is not gated on `profile_requirements` — a descriptive rule must not become a requirements driver. |

Its own family: separate from `QUALITY_*` and from `TAXONOMY_*`, and no existing
code is renamed, narrowed, or reused. See
`openspec/changes/spec-purpose-backfill-gate/`.

### Migration state — not applied, and the check is fenced

`SPEC_PURPOSE_PLACEHOLDER` is ERROR by owner ruling, and the whole spec-purpose
audit is held off the fleet until the backfill has been applied. The flag
`SPEC_PURPOSE_AUDIT_ENABLED` in `scripts/workspace_check.py` is therefore
**False**, and the severity was never softened to WARNING to get there. The
sequencing is enforced by the checker itself, not by a comment and not by an
environment variable: with the flag on, the same unmigrated corpus that is silent
with the flag off reports ERROR — measured on the real fleet, 205 errors become
1,439, the 1,234 markers and nothing else.

Measured on 2026-10-03: 79 repositories carry an `openspec/specs` tree holding
1,958 spec files, of which 1,234 across 71 repositories hold the marker. The
fleet write is a cross-repo source edit requiring explicit authorization, exactly
like the axis migration, and is not applied by the change that ships the tool and
the rule. The sequencing is:

```bash
# 1. review the plan (dry run; writes nothing)
python3 scripts/spec_purpose_backfill.py

# 2. has it been applied? exit 3 while any row is outstanding
python3 scripts/spec_purpose_backfill.py --check

# 3. apply it, then set SPEC_PURPOSE_AUDIT_ENABLED = True in the same commit
python3 scripts/spec_purpose_backfill.py --apply
```

The tool is stdlib-only, dry run is its only default, and `--apply` is its only
write path. Each plan row records the spec, the change, the source used, the
resolved sentence, and the evidence path, so a reviewer can tell which sentences
are the author's own archived prose and which are derived. A change name matching
more than one archive directory is refused, not guessed, and the refusal is
row-level: the other 1,232 rows stay reviewable, while `--apply` stays
all-or-nothing so the fleet is never half-migrated.

Unlike the taxonomy fence, the constant is **not sufficient on its own** to gate
this rule. The `governance-quality` concern in `driftwatchdog` filters its verdict
to the closed allow-list `GOVERNANCE_QUALITY_DECLARED_CODES` and treats the
checker's exit 1 as an *expected* exit, so with this code outside that set the
concern would report `PASS` while the checker fails. Adding
`"SPEC_PURPOSE_PLACEHOLDER"` to that set is a separately authorized cross-repo
step in a repository this change does not own; `SPEC_PURPOSE_AUDIT_ENABLED` must
stay `False` until it lands. Recorded as a blocker in `design.md` §6 rather than
claimed as wiring.
