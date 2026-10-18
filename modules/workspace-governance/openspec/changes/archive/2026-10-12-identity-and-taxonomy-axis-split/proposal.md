# Proposal: Split identity from taxonomy — `role` × `stack`, and an authoritative `id`

## Why

The portfolio has 81 `.project.json` declarations and 78 `projects.json`
entries. Two defects sit underneath that count, and both are invisible to
the current audit because the audit has no vocabulary for either one.

**The taxonomy field conflates two independent questions in one string.**
`kind` is written as a role word and a stack word welded together —
`product` 31, `dotnet-product` 19, `typescript-product` 7, `rust-product` 6,
`tooling` 6, `platform` 3, `shared-library` 3, `python-product` 2, plus four
singletons. "What role does this repository play?" and "what is it written
in?" are then forced into one value, so **role is unrepresentable whenever
the stack is interesting**. The registry has only the conflated strings to
work with, so it collapses back onto the stack:

| Repository | `.project.json` `kind` | registry `profile` | The registry says it is |
|---|---|---|---|
| `forge` | `platform` | `rust-product` | a Rust product |
| `driftwatchdog` | `tooling` | `rust-product` | a Rust product |
| `ariadex` | `tooling` | `python-runtime` | a Python runtime |
| `labrys` | `platform` | `rust-platform` | a Rust platform |
| `alltools-platform` | `platform` | `platform` | a platform |

These are the four repositories whose classification the rest of the
workspace reasons from — the platform, the gate runtime, the Python
tooling, the Rust platform — and they are exactly the ones a stack-shaped
field gets wrong. `forge` and `driftwatchdog` are the providers the
capability-adoption gate binds to; a query for "Rust products" returns the
platform that is supposed to *serve* products.

**The disagreement is the normal case, not an edge case.** **33 of 81**
declarations carry a `kind` that differs from their own `profile`, and the
registry `profile` differs from the declaration `kind` in **30 of the 77**
comparable repositories (39%). Eighteen of the thirty are the same shape:
the author wrote the stack into `kind` (`dotnet-product`) and the bare role
into `profile` (`product`), and the registry faithfully copied `profile` —
so the registry is wrong about the stack for all eighteen while the
declaration is right. `transitfy` is the mirror image: declaration
`typescript-product` against registry `rust-product`, and a bounded census
of its tree finds 56 TypeScript files and no Rust file at all.

**Identity is not authoritative.** `id` is the join key for
`DECLARATION_ID`, capability adoption provider ids, adoption grants, the
decisions register, the promotion ledger, and the Forge observation, yet
nothing states that it is immutable or that it is not a function of `path`.
Meanwhile:

- **4 active repositories are unregistered** — `argoscope`, `chronicleaf`,
  `devloom`, `kairovia` — all four have a valid declaration and real source
  (133–449 files), and all four are invisible to every registry-driven
  check.
- **1 entry is a true ghost** — `jenkins-bootstrap`, `lifecycle: active`,
  `adoption: adopted`, with no directory on disk.
- **8 entries look like ghosts and are not**: `hestia-lab`/`hestiaLab`,
  `lore-mix`/`loreMix`, `lileililiwen-site`/`lileililiwen.github.io`,
  `campus-mesh`/`campusMesh`, `care-bridge`/`careBridge`,
  `open-teacher-kit`/`openTeacherKit`, `retake-kit`/`retakeKit`,
  `trail-crew`/`trailCrew`. All eight resolve through `path`, all eight
  carry a declaration whose `id` already equals the registry `id`, and all
  eight are already correct under the shipped `registry-integrity` rule
  ("canonical id is derived at authoring time"). A name-based dedup or a
  "normalize the id to the directory" sweep would break all eight across
  every key listed above.

The 30 disagreements, the 4 unregistered, and the `kind` rewrite are
already logged as non-goals and deferred items 5.3/5.4 in the in-flight
`capability-adoption-gate` change, which keys its findings on provider id
precisely so it does not inherit the taxonomy defect. This change is that
deferred work.

## What Changes

- **Split the enum into two orthogonal axes.** `role` ∈
  `product | platform | tooling | governance | deployment | inventory` and
  `stack` ∈ `dotnet | rust | typescript | python | flutter | none`, both
  canonical in `vocabulary.json` and enumerated in
  `schemas/project.schema.json`. `kind` becomes a deprecated composite that
  is *derived*, never authored.
- **Make `id` authoritative and immutable.** The registry `id` is the
  identity key; it MUST be kebab-case, MUST NOT change once another
  artifact references it, and MUST NOT be derived from or renamed to match
  `path`. `path` lives only in the registry and is resolved, never
  synthesized. A directory whose name is not a valid identifier records an
  explicit `alias`; the eight `campusMesh`-style entries already do this
  and MUST NOT be churned.
- **Carry both axes in both authority surfaces.** The registry entry gains
  `role` and `stack`; the declaration gains `role` and `stack`. A registry
  entry and its declaration that disagree on either axis is a finding, so
  the two surfaces can no longer drift the way 30 of them have.
- **Ship the migration for the measured drift.** The 30 disagreeing
  repositories, the 4 unregistered repositories, and the 1 ghost entry are
  resolved by an explicit table in `design.md` §5, with the resolution
  source named per row.
- **Declare three finding codes** in the new `TAXONOMY_*` family:
  `TAXONOMY_AXIS_UNKNOWN`, `TAXONOMY_MISMATCH`, `TAXONOMY_LEGACY_VALUE`.
- **Reuse the codes that already exist for the two identity defects.**
  `DISCOVERED_UNREGISTERED` already fires for the four unregistered
  repositories and `PROJECT_MISSING` already fires for `jenkins-bootstrap`;
  this change adds no second code for either, it only removes the cause.

## Package Boundary and Split Assessment

This is one package because the axes and the identity rule are one
vocabulary decision with one oracle: after migration, every one of the 81
declarations carries a canonical `role`/`stack` pair, the 78 registry
entries agree with them, and no registry key has moved. Splitting the axis
split from the identity rule would leave a window in which `id` is
simultaneously authoritative and derived from `path` — exactly the state
that makes the eight camelCase entries dangerous to touch.

| Package | Single outcome | Owner / language | Boundary and contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `identity-and-taxonomy-axis-split` (this) | A project is described by an immutable `id` plus two independent axes, and the registry and declaration agree on both | `workspace-governance`, Python 3.11 stdlib | `vocabulary.json` `roles`/`stacks`; schema `role`/`stack`; registry + declaration fields; `TAXONOMY_*` findings | `registry-integrity` (shipped id/path rule), `portfolio-vocabulary` (shipped canonical sets) | 81 declarations and 78 entries carry canonical axes; the 30 disagreements resolve; the 8 alias entries are byte-identical before and after |
| `capability-adoption-gate` (in flight) | Non-consumption of a registered capability is a measured finding | `workspace-governance`, Python 3.11 stdlib | `capabilities.json`, `CAPABILITY_ADOPTION_*` | provider id, not `kind` | independent; it keys on provider id and is unaffected by the axis split |
| `readme-claim-verification` (companion) | A "not implemented" README claim is checked against the file tree | `workspace-governance`, Python 3.11 stdlib | `product_code_quality.py`, `QUALITY_README_*` | reads the same tree, never the axes | independent; it must not key on `role`/`stack` |

`readme-claim-verification` is a separate package and is *not* merged here
for one concrete reason: the two have opposite blast radii. Reclassifying a
repository is a governance-owned, workspace-local edit that this owner may
make. Correcting a false README claim is a source edit to a sibling
repository, which `AGENTS.md` forbids without scope authorization — the same
permission gap that split `capability-adoption-gate` from
`cross-repo-adoption-grant`. Bundling them would let a taxonomy migration
become a fleet-wide source edit.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `validate_registry` | `scripts/workspace_check.py:275` | id kebab rule, duplicate id/path detection, `PROJECT_MISSING` for the ghost | Reads one `profile` string; no axis awareness | workspace-governance | **extend shared owner** (additive `role`/`stack`, same signature style) |
| `validate_kind` | `scripts/workspace_check.py:372` | the single existing place a declaration's self-description is checked; WARNING-level and explicitly non-requirement-bearing | Checks membership in one set; cannot express role ≠ stack | workspace-governance | **replace** with `validate_taxonomy`; `kind` becomes derived |
| `profile_requirements` | `scripts/workspace_check.py:319` | the only consumer that turns classification into requirements | Reads the registry `profile` and MUST NOT start reading `role`/`stack` | workspace-governance | **keep unchanged** — required-file outcomes are frozen by this change |
| `vocabulary.json` | `profiles` (15), `kinds` (12) | the single machine-readable vocabulary all consumers already load | Has no axis sets; `kinds` is the composite being retired | workspace-governance | **extend shared owner** (additive `roles`/`stacks`) |
| `schemas/project.schema.json` | `required: [schema_version, id, kind, profile, lifecycle, verification, deployment]`, `kind` description | versioned declaration contract, v1 | `kind`/`profile` are required strings; no axis fields | workspace-governance | **extend additively**; `kind` stays accepted as deprecated, never newly required |
| `init_project.py` | canonical-id derivation + explicit alias path (shipped `registry-integrity`) | the authoring-time id rule this change makes authoritative | Derives id from the directory and refuses a differing `--id` | workspace-governance | **adopt unchanged**; this change only states that the derived id is then immutable |
| `platform-contracts` schema registry | `schemas/registry.json` | a language-neutral contract registry | Describes the registry file, not project classification | platform-contracts | **defer**; classification stays governance-owned |
| `capabilities.json` | `provider` ids, `expected_since` | capability identity already keyed on repo id | Contains no `kind`/`role` read | workspace-governance | **no change**; bindings stay valid because ids do not move |
| `decisions.json` | `validate_against_registry(loaded, registry_ids)` | rejects references to unknown registry ids | Breaks if any id is renamed | workspace-governance | **must stay green** — the immutability oracle |
| `deployment/promotions.json` | `scripts/promote_project.py` | append-only ledger keyed by project id | Breaks if any id is renamed | workspace-governance | **must stay green** — the immutability oracle |
| `AGENTS.md` (7 files, 6 repos + template) | "Assess shared infrastructure before duplication" | the instruction that makes classification consequential | An agent resolving a sibling needs the role, and today gets a stack word | each product | **document the new axes in `docs/vocabulary.md` only**; no sibling edit in this change |

No sibling repository owns a project-classification vocabulary, so no
extraction is authorized. `workspace-governance` owns it; the four platform
repositories consume it by declaring the same two values.

## BFS Impact Map

- **Actors:** the governance auditor (read-only), the registry owner
  (resolves a disagreement and records the resolution), a product
  repository (declares its own `role`/`stack`).
- **State:** a project moves from *one composite string* to *two
  independent values*. A repository entry moves
  `composite → axes-resolved`; an entry whose axis pair disagrees with its
  declaration moves `claimed → agreed` when the owner adopts one of the two
  values. A registry entry moves `registered → unregistered-removed` for
  the ghost, and `unregistered → registered` for the four active repos.
- **Contracts:** `vocabulary.json` gains `roles` and `stacks`; the
  declaration schema gains `role` and `stack` and marks `kind` deprecated;
  the registry entry gains `role` and `stack`; `kind` gains a documented
  derivation `kind = stack + "-" + role` (or `role` when `stack` is `none`)
  so any consumer that still reads the composite gets a value it can parse.
- **Data integrity:** `id` is unchanged for all 78 existing entries. The
  migration MUST be asserted id-for-id against the pre-change registry: any
  id that appears, disappears, or changes spelling is a failed migration, not
  a rename. `decisions.json` and `deployment/promotions.json` are read-only
  oracles for the same property.
- **Failure and boundary:** a `role`/`stack` value outside the canonical set,
  or absent, is WARNING `TAXONOMY_AXIS_UNKNOWN` naming both canonical sets
  — WARNING, because both axes are descriptive and a new repository may
  legitimately precede a vocabulary addition. A registry/declaration
  disagreement on either axis is ERROR `TAXONOMY_MISMATCH` naming both
  values and both ids, because two authority surfaces are asserting
  different facts about the same repository. A remaining stack-qualified
  composite is WARNING `TAXONOMY_LEGACY_VALUE`, which retires when both new
  fields are present. A missing or unparseable `vocabulary.json` is a
  hard non-zero exit, never an empty-set fallback — the same rule the
  shipped vocabulary change established, because an empty axis set would
  make every repository an unknown.
- **Compatibility:** `profile_requirements` reads the registry `profile`
  only, so no project's required-file, CI, verification, or OpenSpec
  outcome may change. `KIND_UNKNOWN` keeps its existing meaning for a
  `kind` outside the 12 canonical kinds; `TAXONOMY_LEGACY_VALUE` covers the
  stack-qualified case. `workspace_check.py` still does not promote a
  capability to verified on its own, and the new axes never feed
  `measure_capability_adoption`.
- **Security:** the classification comes from two JSON files the owner
  already edits. No project command, no file-tree walk, no network. A
  census is used **once, offline, as a migration aid** and its result is
  advisory; the audit never infers an axis from the tree, which keeps the
  invariant that a declaration is intent, not evidence.
- **Tests:** canonical-set membership for both axes; the `kind` derivation
  round-trip; a fixture reproducing the 30-repository disagreement set; a
  fixture proving the 8 alias entries produce no taxonomy finding; a
  registry-immutability test comparing ids before and after migration; and a
  test proving the axes do not change any `profile_requirements` outcome.
- **Dependencies:** Python 3.11 standard library only.

## Capabilities

- **New:** `identity-and-taxonomy`.
- **Modified:** `portfolio-vocabulary` — `kinds` is retired as an authored
  value and re-expressed as two canonical arrays. Its existing requirement
  that the vocabulary is a single machine-readable source, and its
  hard-fail-on-missing rule, are preserved and extended, not weakened.
- **Modified:** `registry-integrity` — the shipped
  "canonical id is derived at authoring time" requirement is extended with
  immutability, so that the eight `campusMesh`-style entries are explicitly
  *not* findings and must not be normalized.

## Non-goals

- No change to any sibling repository's source, README, or CI. This change
  edits `projects.json` and the 81 `.project.json` files only, and only in
  their classification fields.
- No change to `profile_requirements`, to which files a profile requires, or
  to any repository's current required-file outcome.
- No change to `projects.json` entry ids, and no rename of
  `hestia-lab`, `lore-mix`, `lileililiwen-site`, `campus-mesh`,
  `care-bridge`, `open-teacher-kit`, `retake-kit` or `trail-crew`. Their
  `path` values are already correct and are the resolution mechanism.
- No new finding code for the four unregistered repositories or the ghost:
  `DISCOVERED_UNREGISTERED` and `PROJECT_MISSING` already fire.
- No inference of `role` or `stack` from the file tree at audit time. The
  census is a one-time migration aid, never an audit signal.
- No merge of the axis split with `capability-adoption-gate`,
  `cross-repo-adoption-grant`, or `readme-claim-verification`.
- No `layout` axis for `typescript-monorepo`, and no `stacks` or
  `secondary_stacks` array for the three repositories that ship a .NET API
  plus a Flutter client. Both are now decided non-values rather than open
  questions: the axes stay orthogonal and `stack` stays single-valued, with
  the revisit trigger recorded in ruling 3 below.
- No promotion of any repository to `verified`, `deployable`, or any other
  evidence state. A declaration remains intent, not evidence.

## Owner rulings — decided

The owner has ruled on all five questions this section previously held open.
The rulings are binding on the vocabulary, the migration table, and the
finding levels. The corresponding record in design terms is `design.md` §10.

1. **`shared-library` is not a seventh role.** `platform-contracts`,
   `rust-platform-libs` and `dotnet-platform-libs` fold into
   `role: platform`, and the role set stays at six values. No consumer
   branches on the distinction: the capability-adoption gate keys on provider
   id, and `profile_requirements` is frozen and never reads `role`. The
   distinction that does carry weight — a library you consume versus a service
   you run — is already expressed by the frozen `profile` values
   `dotnet-library` and `rust-platform`, so a seventh role would duplicate a
   frozen axis instead of adding one. The registry's own three-way
   disagreement across exactly these three repositories — `tooling`,
   `rust-platform` and `dotnet-library` — is recorded as **a defect of the
   composite, not a reason for a new value**.
2. **`TAXONOMY_MISMATCH` is ERROR**, under a binding sequencing constraint:
   the check MUST NOT be enabled until the `design.md` §5 migration table has
   been applied. The same package resolves all 30 disagreements, so on day
   one the same 30 disagreements the check exists to catch are already
   resolved and an ERROR enabled before the migration never fires on the
   fleet — it would be recorded as passing on the exact condition it was
   written to detect. Check and migration ship together; if they cannot, the
   severity is WARNING until they do.
3. **`stack` stays single-valued.** `lexora`, `opendockify` and `trippify`
   record `stack: dotnet` — the implementation stack — and no `stacks` array
   is added. The justification is that a multi-valued axis is a schema and
   vocabulary change with no current consumer. **Revisit trigger:** a
   `stacks` array is added only when a real consumer needs to query "which
   repositories ship a Flutter client". The fact is already captured
   machine-readably today: `forge` declares `flutter-app: tokens: none`.
4. **`lileililiwen-site` is `role: deployment`.** It is a Hugo static site:
   `verification.command` is `hugo --gc --minify`, it carries `CNAME` files
   under `static/` and `public/`, it holds 16 `.html` and 12 `.py` files, it
   has no application entry point, no compose file and no `Dockerfile`, and
   it has a `deploy.yml` workflow. It ships no runnable artifact, so it is
   not a `product`. Both authority surfaces already agree on
   `profile: deployment`, so following that existing agreement introduces no
   new vocabulary. This resolves a `TAXONOMY_MISMATCH` — declaration
   `kind: product` against `profile: deployment` on both surfaces — and is
   explicitly **not** a vocabulary change. The repository lives on disk at
   `lileililiwen.github.io` through its registry `path` and is one of the
   eight alias entries, so its `id` MUST NOT be renamed.
5. **`kind` is retired as an authored value and retained permanently as a
   derived, deprecated field.** The derivation is `kind = stack + "-" + role`,
   or the bare `role` when `stack` is `none`. `kind` is in the schema's
   `required` list and appears in all 81 declarations, so removing it would
   require 81 authored-declaration edits that this change's own non-goals
   forbid. Deriving it costs one function and keeps every undocumented
   consumer of the composite working.
