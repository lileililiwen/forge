# Design: identity-and-taxonomy

## 1. Implementation boundary

**Repository:** `/home/paul/code/workspace-governance` — Python 3.11, standard
library only. No sibling repository source is edited.

**Files to change:**

- `vocabulary.json` — add `roles` and `stacks`; keep `kinds` (deprecated,
  derived) and every existing array byte-identical.
- `schemas/project.schema.json` — add `role` and `stack` to `properties`,
  add matching `$defs`, and update the `kind` description to state that it
  is derived and deprecated. `required` is **not** extended: a declaration
  without the axes stays valid and reports `TAXONOMY_AXIS_UNKNOWN`.
- `scripts/workspace_check.py` — add `canonical_roles_for` /
  `canonical_stacks_for` accessors, add `validate_taxonomy`, extend
  `validate_registry` with the two axis fields, and call
  `validate_taxonomy` from `validate_project` where `validate_kind` is
  called today. Declare `TAXONOMY_AXIS_UNKNOWN`, `TAXONOMY_MISMATCH`,
  `TAXONOMY_LEGACY_VALUE`.
- `projects.json` — add `role` and `stack` to all 78 entries; add the 4
  unregistered repositories; remove or archive the 1 ghost. **No `id` and
  no `path` value changes.**
- The 81 `.project.json` files — add `role` and `stack`; leave `kind` in
  place as the derived composite.
- `scripts/vocabulary.py` — surface `roles` and `stacks` with the existing
  fail-closed duplicate/empty handling.
- `docs/vocabulary.md`, `README.md` — document the two axes; keep the
  single-entry-point rule (link, do not restate).
- `tests/test_identity_and_taxonomy.py` (new).

**Must not change:** `profile_requirements`, `profiles/*.json`, the
`capabilities` / `consumes` / `declines` / `release_evidence` / `standalone`
schema blocks, `capabilities.json`, `decisions.json`,
`deployment/promotions.json`, `scripts/promote_project.py`,
`scripts/init_project.py`, `scripts/forge_governance_adapter.py`,
`auto_gate.py`, and any sibling repository.

## 2. Language and runtime

Python **3.11**, standard library only.

- `python3 -m unittest discover -s tests -v`
- `python3 scripts/workspace_check.py --root .. --audit --json`
- `python3 scripts/workspace_check.py --self-check`
- `python3 scripts/maturity_audit.py --root .. --json`
- `node scripts/check-openspec-change-names.mjs`
- `openspec validate identity-and-taxonomy-axis-split --strict`
- `git diff --check`

## 3. Ownership and shared code

- **Owner:** `workspace-governance`. No sibling owns project
  classification, so no extraction is authorized.
- **Exported contract:** `vocabulary.json` `roles` and `stacks`.
- **Consumers:** `workspace_check.py` (audit), `init_project.py` (authoring
  hint only — it does not need to read the axes to derive an id), and any
  product repository that wants to know a sibling's role before duplicating
  a capability.
- **Dependency direction:** products → governance.
- **Release boundary:** ships with this repository. `platform-contracts`
  reconciles its schema registry separately.

## 4. The two-axis model

`kind` currently answers two questions at once. The split separates them.

| `role` | Meaning | Repositories today (from `kind`) |
|---|---|---|
| `product` | Ships a user-facing or business capability of its own | 66 |
| `platform` | Exists to be consumed by other repositories | 6 (3 authored `platform` + 3 `shared-library` repositories folded in by owner ruling, §10 ruling 1) |
| `tooling` | Internal or agent-facing tool, not a shipped product | 6 |
| `governance` | Owns workspace policy, rules, and registries | 1 (`workspace-governance`) |
| `deployment` | Owns build, release, and runtime plumbing, not a product | 1 (`jenkins-local`) |
| `inventory` | Records the portfolio; ships no implementation | 1 (`catchen`) |
| `shared-library` | **Not a role** — folded into `platform` by owner ruling; the library-versus-service distinction is already carried by the frozen `profile` values | 3 (decided, §10 ruling 1) |

| `stack` | Meaning | Repositories today (resolved by §5) |
|---|---|---|
| `dotnet` | C#, F#, or VB implementation | 45 |
| `rust` | Rust implementation | 17 |
| `typescript` | TypeScript/JavaScript implementation | 9 |
| `python` | Python implementation | 9 |
| `flutter` | Dart/Flutter implementation | 1 |
| `none` | No implementation language — contracts, documents, schemas only | 0 today; required for `platform-contracts` if the owner prefers it (§5) |

**Orthogonality invariant.** `role` and `stack` are independent: every
combination is legal, including `role: platform, stack: rust` (which is
`forge` today) and `role: tooling, stack: rust` (`driftwatchdog`). A
combination that is currently inexpressible — "a platform written in Rust"
being recorded as anything other than a Rust product — is the defect this
change removes.

**Derivation of the legacy composite.** For any consumer that still reads
`kind`, the derived value is `f"{stack}-{role}"` when `stack` is not `none`
and `f"{role}"` when it is. Round-trip MUST be exact:
`product`+`dotnet` → `dotnet-product` → (`product`, `dotnet`). This keeps
the shipped 12-kind set meaningful as a *derived* projection of 6 × 6 pairs
rather than as an authored enum.

**Identity invariant.** `id` is the immutable join key; `path` is a
resolution detail that exists only in the registry. `id` is never computed
from `path` at audit time. Where the directory name is not a valid
identifier, the registry entry already carries the kebab `id` and the real
`path`, and the declaration repeats the `id` — that arrangement is correct
and is not a finding.

## 5. Migration

### 5.1 Stack resolution order (deterministic)

Applied in order; the first rule that yields a value wins, and the rule used
is recorded per row so a reviewer can see *why* a row resolved the way it
did.

1. **Stack-qualified `kind`** — `dotnet-product`, `rust-product`,
   `typescript-product`, `python-product` → the prefix. The declaration's
   own kind is the most specific statement the repository makes.
2. **Stack-qualified declaration `profile`** — the second-most-specific.
3. **Stack-qualified registry `profile`** — the third.
4. **Bounded offline census** — majority extension over a bounded,
   read-only walk, excluding `node_modules`, `dist`, `build`, `target`,
   `bin`, `obj`, `vendor`, `coverage`, `.git`, `__pycache__`, `.venv`,
   `.dart_tool`, `.next`, `out`, `.turbo`. **Advisory only.** Its result is
   written into the declaration by a human, and the audit never re-derives
   an axis from the tree.
5. **`none`** — recorded explicitly, never as a silent fallback.

**Role** is taken from the role word of the stack-qualified `kind` (or the
bare `kind`); a stack-qualified `profile` never overrides the declaration's
role. This is what makes `transitfy` resolve to `(product, typescript)`
against a registry that says `rust-product` — the bounded census finds 56
TypeScript files and no Rust file.

### 5.2 The 30 disagreeing repositories

Registry `profile` differs from declaration `kind` in 30 of 77 comparable
repositories. Resolved target for each:

| Repository | `kind` | registry `profile` | Resolved `role` | Resolved `stack` | Rule |
|---|---|---|---|---|---|
| `aesthra` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `ariadex` | `tooling` | `python-runtime` | `tooling` | `python` | 2 |
| `crossalheart` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `crossify` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `cvunify` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `dotnet-platform-libs` | `shared-library` | `dotnet-library` | `platform` | `dotnet` | 2 — decided, §10 ruling 1 |
| `driftwatchdog` | `tooling` | `rust-product` | `tooling` | `rust` | 2 |
| `forge` | `platform` | `rust-product` | `platform` | `rust` | 2 |
| `fotofy` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `hermora` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `hestiaLab` (`hestia-lab`) | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `hookit` | `product` | `rust-product` | `product` | `rust` | 3 — census: 87 `.rs` |
| `hypora` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `kairion` | `product` | `dotnet-product` | `product` | `dotnet` | 2 |
| `labrys` | `platform` | `rust-platform` | `platform` | `rust` | 2 |
| `lexora` | `dotnet-product` | `product` | `product` | `dotnet` | 1 — also ships 24 `.dart`; single-valued `stack`, §10 ruling 3 |
| `lileililiwen.github.io` (`lileililiwen-site`) | `product` | `deployment` | `deployment` | `none` | 4 — decided, §10 rulings 4 and 9; the census counts build-time validators, not shipped code, so `stack` is `none` |
| `mortalect` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `opendockify` | `dotnet-product` | `product` | `product` | `dotnet` | 1 — also ships 75 `.dart`; single-valued `stack`, §10 ruling 3 |
| `platform-contracts` | `shared-library` | `tooling` | `platform` | `python` or `none` | 4 — decided, §10 ruling 1; 14 `.py` tooling, JSON-schema artifact |
| `ploutify` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `rust-platform-libs` | `shared-library` | `rust-platform` | `platform` | `rust` | 2 — decided, §10 ruling 1 |
| `singleatee` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `sisyphusfy` | `tooling` | `python-product` | `tooling` | `python` | 2 |
| `somodanote` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `stylify` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `sutravia` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `tantalyn` | `dotnet-product` | `product` | `product` | `dotnet` | 1 |
| `transitfy` | `typescript-product` | `rust-product` | `product` | `typescript` | 1 — registry stack is wrong; 56 `.ts`, 0 `.rs` |
| `trippify` | `dotnet-product` | `product` | `product` | `dotnet` | 1 — also ships 23 `.dart`; single-valued `stack`, §10 ruling 3 |

Nineteen of the thirty are rule-1 rows where the declaration is right about
the stack and the registry silently lost it — eighteen of those nineteen are
the pure `kind: <stack>-product` / registry `product` shape; the remaining
eleven are rule-2/3/4 rows where the declaration `profile`, the registry
`profile`, or the census carries the only stack statement.

### 5.3 The 4 unregistered repositories

All four have a valid schema-v1 declaration, a real source tree, and
currently report ERROR `DISCOVERED_UNREGISTERED`. Migration adds the
registry entry; no new code is introduced for them.

| Directory | `kind` | declaration `profile` | Resolved `role` | Resolved `stack` | Rule | Bounded census |
|---|---|---|---|---|---|---|
| `argoscope` | `product` | `dotnet-product` | `product` | `dotnet` | 2 | 133 `.cs` |
| `chronicleaf` | `product` | `dotnet-product` | `product` | `dotnet` | 2 | 133 `.cs`, 26 `.ts` |
| `devloom` | `product` | `dotnet-product` | `product` | `dotnet` | 2 | 449 `.cs`, 129 `.ts` |
| `kairovia` | `product` | `dotnet-product` | `product` | `dotnet` | 2 | 240 `.cs`, 98 `.ts` |

### 5.4 The 1 ghost entry

`jenkins-bootstrap` is `lifecycle: active`, `adoption: adopted`,
`profile: deployment`, and has no directory on disk. It already reports
ERROR `PROJECT_MISSING` from `validate_registry`. The migration removes or
archives the entry; it does not invent a second code, and it does not
recreate the directory.

### 5.5 The 8 alias entries — explicitly not churned

| Registry `id` | `path` | State |
|---|---|---|
| `campus-mesh` | `campusMesh` | correct; already conformant |
| `care-bridge` | `careBridge` | correct; already conformant |
| `hestia-lab` | `hestiaLab` | correct; already conformant |
| `lore-mix` | `loreMix` | correct; already conformant |
| `lileililiwen-site` | `lileililiwen.github.io` | correct; already conformant |
| `open-teacher-kit` | `openTeacherKit` | correct; already conformant |
| `retake-kit` | `retakeKit` | correct; already conformant |
| `trail-crew` | `trailCrew` | correct; already conformant |

Every one of the eight already has a kebab-case registry `id`, a
resolvable `path`, and a declaration whose `id` equals the registry `id`.
They MUST be byte-identical before and after this change except for the two
newly added axis fields. Renaming the `id` to match the directory, or the
`path` to match the `id`, breaks `DECLARATION_ID`,
`measure_capability_adoption` provider bindings, the decisions register
reference check, and the promotion ledger — which is why the immutability
assertion in §7 is an id-set comparison, not a spot check.

## 6. Contract and compatibility

- `vocabulary.json` gains `roles` (6) and `stacks` (6). `schema_version`
  stays `1`; the change is additive, so no existing consumer sees a diff.
- New codes, all in a new `TAXONOMY_*` namespace that collides with nothing
  in use (`CAPABILITY_*`, `ADOPTION_*`, `STANDALONE_*`, `QUALITY_*`,
  `RELEASE_EVIDENCE_*`, `DECISIONS_*`, `DEPLOYMENT_*`, `SECRET_*`):

| Code | Level | Fires when |
|---|---|---|
| `TAXONOMY_AXIS_UNKNOWN` | WARNING | `role` or `stack` is absent or outside the canonical set; message names both sets |
| `TAXONOMY_MISMATCH` | ERROR | registry `role`/`stack` disagrees with the declaration's, naming both values and both keys |
| `TAXONOMY_LEGACY_VALUE` | WARNING | a stack-qualified `kind` or `profile` composite remains while the axes are present; retires when both new fields are set |

- `KIND_UNKNOWN` is **not** repurposed. It keeps its shipped meaning (a
  `kind` outside the 12 canonical kinds) so no existing expectation changes.
  `PROFILE_UNKNOWN`, `PROFILE_DOC_UNKNOWN`, `PROFILE_MISSING`,
  `PROJECT_ID`, `PROJECT_DUPLICATE`, `PROJECT_PATH`,
  `PROJECT_PATH_DUPLICATE`, `PROJECT_MISSING`, `DISCOVERED_UNREGISTERED`,
  and `DECLARATION_ID` are untouched.
- `profile_requirements` keeps reading the registry `profile` only. The axes
  are descriptive: they MUST NOT change a required file, a CI requirement,
  an OpenSpec requirement, or a verification requirement for any
  repository. This is the compatibility oracle for the whole change.
- `kind` remains in the schema as a deprecated derived field so that an
  undocumented consumer of the composite keeps parsing. It is never newly
  required and never authored by hand.
- A declaration is intent, not evidence. Adding axes MUST NOT change
  `evidence_status`, `deployable`, or any promotion state, and
  `workspace_check.py` still does not promote a capability to verified on
  its own.

## 7. Failure and boundary policy

| Case | Result |
|---|---|
| `vocabulary.json` missing, unparseable, or with an empty `roles`/`stacks` | hard non-zero exit naming the file and the problem; never an empty-set fallback, which would mark all 81 repositories unknown |
| duplicate entry inside `roles` or `stacks` | ERROR at load; the vocabulary must be unambiguous |
| `role`/`stack` absent | WARNING `TAXONOMY_AXIS_UNKNOWN`; the declaration remains schema-valid |
| `role`/`stack` non-canonical | WARNING `TAXONOMY_AXIS_UNKNOWN` naming the value and both sets |
| registry and declaration disagree on one axis | ERROR `TAXONOMY_MISMATCH` naming both values |
| `TAXONOMY_MISMATCH` enabled before the §5 migration is applied | sequencing violation. ERROR is valid only in the commit that also applies §5; until then the level is WARNING, because on day one the same package resolves all 30 disagreements and the check would never fire on the fleet (§10 ruling 2) |
| registry has an axis, declaration does not | WARNING `TAXONOMY_AXIS_UNKNOWN` on the declaration, not a mismatch — an absent value is not a contradiction |
| stack-qualified composite still present with both axes set | WARNING `TAXONOMY_LEGACY_VALUE`; a hand-authored `kind` that disagrees with its own derived form is the same code, because the derived form is the authority |
| `id` present but not kebab-case | existing ERROR `PROJECT_ID`; unchanged |
| `id` renamed by the migration | **migration failure.** The id set must compare equal before and after; `decisions.json` and `deployment/promotions.json` must stay green |
| `path` does not resolve | existing ERROR `PROJECT_MISSING`; unchanged. An `id`/`path` spelling difference is **not** this error |
| `id` equals `path` | no finding; the two being equal is a coincidence, not a requirement |
| census disagrees with every declared value | no finding — the census is a migration aid and is not an audit signal |

## 8. Verification oracle

- `python3 -m unittest discover -s tests -v` — all pass, including: axis
  canonical-set enforcement for registry and declaration; the `kind`
  derivation round-trip; the 30-repository disagreement fixture resolving to
  the §5.2 table; the 4 unregistered repositories reporting
  `DISCOVERED_UNREGISTERED` before their entries exist and no finding
  after; and the 8 alias entries producing no taxonomy finding.
- **Id immutability:** a direct comparison of the registry id set before and
  after the migration returns an empty difference, and
  `decisions.json` / `deployment/promotions.json` remain valid against it.
- **No-outcome-change proof:** for every one of the 78 registry entries,
  `profile_requirements(profile)` returns the identical tuple before and
  after, and the audit's `REQUIRED_FILE`, `CI_MISSING`,
  `OPENSPEC_MISSING` and `VERIFICATION_COMMAND` findings are unchanged.
- `python3 scripts/workspace_check.py --root .. --audit --json` — 0 errors
  and no `TAXONOMY_*` finding once the migration lands; the pre-migration
  run is expected to show exactly 30 `TAXONOMY_MISMATCH` errors and is
  recorded, not hidden.
- `python3 scripts/workspace_check.py --self-check` — 0 errors.
- `python3 scripts/maturity_audit.py --root .. --json` — counts agree with
  the audit (the single authoritative result invariant).
- `node scripts/check-openspec-change-names.mjs` and
  `openspec validate identity-and-taxonomy-axis-split --strict` — pass.
- `git diff --check` — clean.
- Record actual outputs. Any environment-blocked check is reported with its
  exact next action, never as passing.

## 9. Decision ledger

**Assumptions:** `kind`/`profile` are descriptive; the registry `profile`
is the sole driver of requirements and stays so; the declaration `kind` is
the repository's most specific self-statement and therefore outranks both
`profile` values when they conflict; an absent axis is not a contradiction.

**Resolved alternatives:**

- (a) Rewrite `kind` in place vs add two axes → chose **two axes with a
  derived `kind`**. An in-place rewrite would either reject the 30
  disagreeing repositories outright or silently pick a winner, and it would
  break every consumer of the composite string.
- (b) One `taxonomy` object vs two top-level fields → chose **two top-level
  fields** (`role`, `stack`). A nested object invites consumers to treat it
  as optional as a unit, which is exactly the conflation being removed.
- (c) ERROR vs WARNING for an axis mismatch → chose **ERROR**, with the
  pre-migration count recorded honestly. WARNING would let the disagreement
  class persist, which is the defect.
- (d) New codes for the unregistered repositories and the ghost → chose
  **reuse** `DISCOVERED_UNREGISTERED` and `PROJECT_MISSING`. Two codes for
  one condition is how a fleet ends up with two contradictory reports of
  the same defect.
- (e) Infer the axes from the file tree at audit time → chose **no**. The
  census is a one-time migration aid. A tree-derived axis would make a
  declaration's own statement irrelevant, contradicting the invariant that
  a declaration is intent, not evidence.
- (f) Fold the README-claim change in → chose **separate package**; see the
  package boundary table.
- (g) Remediate the 8 `campusMesh`-style entries → chose **leave them
  untouched**. They are already correct and are what the immutable-id rule
  looks like in practice.

**Deferred:** a multi-stack `stacks` array, revisited only on the trigger in
§10 ruling 3; a `layout` axis for `typescript-monorepo`;
any `platform-contracts` schema-registry alignment; a possible
`ROLE_*`/`STACK_*` reporting surface in `maturity_audit.py` (display only,
and only after the axes are stable).

**Blockers:** none. The owner has ruled on the five questions this change
held open (§10), so the three `shared-library` rows and `lileililiwen-site`
are decided and their `role` values are written per rulings 1 and 4 rather
than held for a later correction. The one remaining ordering requirement is
not a blocker: §10 ruling 2 makes the migration and the enabled
`TAXONOMY_MISMATCH` check a single commit, so the axes are still
descriptive and nothing depends on an unanswered question.

## 10. Owner rulings — decided

The owner has ruled on the five questions this change held open. The rulings
are binding on the vocabulary, the §5 migration table, and the finding
levels. The rationale is recorded in `proposal.md` "Owner rulings — decided";
this section records the same rulings in design terms.

| Ruling | Decision | Effect on this design |
|---|---|---|
| 1 | `shared-library` is not a role | §4 keeps six roles; the three `shared-library` repositories take `role: platform` in §5.2, so the `platform` count is 6. The library-versus-service distinction stays in the frozen `profile` values `dotnet-library` / `rust-platform`, so no axis is added. The registry's three-way disagreement across exactly these three repositories (`tooling`, `rust-platform`, `dotnet-library`) is recorded as a defect of the composite, not as a reason for a new value. |
| 2 | `TAXONOMY_MISMATCH` is ERROR | §6 and §7 keep ERROR, conditional on the sequencing constraint below. The two rows that resolved the `shared-library` composite and the `lileililiwen-site` row are no longer flagged contested. |
| 3 | `stack` stays single-valued | `lexora`, `opendockify` and `trippify` keep `stack: dotnet` in §5.2. No `stacks` array and no `secondary_stacks` field. Revisit trigger: a real consumer querying "which repositories ship a Flutter client"; `forge`'s `flutter-app: tokens: none` already carries that fact machine-readably. |
| 4 | `lileililiwen-site` is `role: deployment` | §5.2 writes `deployment`; the declaration `kind: product` against `profile: deployment` disagreement on both surfaces is resolved, not carried as contested. The ruling fixes the **role**; the row's `stack` was first left as rule 4 resolved it and was then corrected to `none` by ruling 9, which supersedes only the stack half of this ruling. No vocabulary value is added, and the entry stays one of the eight §5.5 alias entries: id `lileililiwen-site`, path `lileililiwen.github.io`, not renamed. |
| 5 | `kind` is retired as authored, retained as derived | The §4 derivation is permanent, not deferred: `stack + "-" + role`, or the bare `role` when `stack` is `none`. `kind` remains in the schema `required` list and remains present in all 81 declarations, derived by the checker and never hand-authored, so no consumer of the composite breaks. |

| 6 | The migration also writes the derived `kind` | Ruling 5 makes `kind` never hand-authored, so §5's migration is the one pass that stops it being hand-authored. The previous hand-authored value is preserved as `declared_kind` on each committed plan row, so the measurement survives the write. Measured: 47 of 81 declarations carry a hand-authored `kind` that differs from the derived value. Leaving them stale would put 81 permanent `TAXONOMY_LEGACY_VALUE` warnings on the fleet, and a warning nobody can ever clear is noise that teaches people to ignore the checker — the exact failure class ruling D18 names. |
| 7 | The §5.2 drift guard accepts the derived `kind` as well as the decided one | Consequence of ruling 6. Without it the tool would reject its own output on the next run — "the fleet has drifted" about the `kind` it had just written — and the migration could never be re-run or verified as applied. Any third value is still refused. Idempotence is proved by a test that applies, checks, applies again and checks again. |
| 8 | `platform-rust` was a typo, not a second derivation | The requirement text and §4 both specify `<stack>-<role>`, and §5.1 rule 1 lists only stack-qualified forms (`dotnet-product`, `rust-product`). The scenario that named `platform-rust` for `role: platform, stack: rust` contradicted both; `rust-platform` is both the derived value and a real member of the shipped 12-kind set. The scenario is corrected. `forge` now demonstrates it: `platform` + `rust` derives `rust-platform`. |
| 9 | The `lileililiwen-site` `stack` is `none`, not `python` | Ruling 4's stack half rested on the bounded census, and §5.1 rule 4 already made that census advisory: "its result is written into a declaration by a human, and the audit never re-derives an axis from the tree." The owner has now ruled the census result wrong for this repository. `lileililiwen.github.io` is a Hugo static site: it ships no implementation language, and every `.py` in the tree is a build-time content validator, so a majority-extension census cannot separate shipped code from tooling. The declaration and the registry were corrected to `stack: none` on 2026-10-05 (sibling `e9cc5df`, then `9bab1b7`); §5.2 is corrected to match. Re-measured 2026-10-05: `python: 16`, `typescript: 1` — the count grew from 12 because the validator set grew, and the conclusion does not change with it. The derived `kind` is the bare `deployment`, which is what ruling 5 specifies for `stack: none`, and the §5.2 drift guard already accepts it under ruling 7. This ruling adds no vocabulary value, changes no other row, and names no sibling that was written. |
**Sequencing constraint (ruling 2).** The §5 migration and the enabled
`TAXONOMY_MISMATCH` check ship in the same commit. Enabling the check
first resolves nothing on day one — this same package resolves all 30
disagreements — so an ERROR enabled before the migration would report zero
fleet findings while the defect is still present, and the check would be
recorded as passing on the exact condition it exists to catch. If the two
cannot ship together, the severity is WARNING until they do.

**Conflicts recorded, not silently resolved.**

- §8 expects "exactly 30 `TAXONOMY_MISMATCH` errors" in the pre-migration
  run, while ruling 2 forbids enabling the check before the migration. Both
  statements are kept as written: the 30 is the measured disagreement count
  and stands as the baseline for the fixture and the audit comparison, and
  the ruling constrains when the check is switched on, not what the count is.
  An implementer must not read the pre-migration run as a sanctioned state in
  which to merge the finding.
- `tasks.md` §0 still records the three `shared-library` rows and
  `lileililiwen-site` as "proposed … the owner must say yes". That caveat is
  superseded by rulings 1 and 4; `tasks.md` was not edited in this pass and
  needs a one-line follow-up before implementation starts.
- Ruling 1 keeps the composite `kind` in the schema `required` list, so the
  vocabulary defect (one string answering two questions) survives as a
  derived projection. That is the accepted cost of ruling 5, and it is why
  `TAXONOMY_LEGACY_VALUE` and the round-trip test remain in force.

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| R1 Two orthogonal axes | `role` × `stack` | §4 | `vocabulary.json`, schema, `validate_taxonomy` | canonical / unknown / absent | 2.1, 2.2 | audit reports no `TAXONOMY_AXIS_UNKNOWN` |
| R2 Derived `kind` | composite survives as a projection | §4 derivation | schema description, `validate_taxonomy` | round-trip / hand-authored drift | 2.3 | round-trip test passes |
| R3 Authoritative immutable `id` | id is the join key, path is resolution | §4, §5.5 | `validate_registry` (unchanged), migration | `id` ≠ `path` / renamed id | 1.3, 2.6, 3.2 | id set unchanged; `decisions.json` green |
| R4 Registry and declaration agree | mismatch is a finding | §6, §7 | `validate_registry`, `validate_project` | agree / disagree / partial | 2.4 | pre-migration 30 `TAXONOMY_MISMATCH`, post-migration 0 |
| R5 Portfolio migrated | 30 + 4 + 1 resolved | §5 | `projects.json`, 81 declarations | per-row resolution | 2.5, 2.6 | direct scan returns no unresolved row |
| R6 No behavioural change | axes are descriptive | §6 | `profile_requirements` untouched | outcome per project | 3.1, 3.3 | identical `profile_requirements` tuples |
