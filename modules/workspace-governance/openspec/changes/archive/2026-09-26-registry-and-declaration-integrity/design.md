# Design: Registry and declaration integrity

## 1. Implementation boundary

**Repository:** `/home/paul/code/workspace-governance` — Python 3.11, standard
library only.

**Files to change (source):**

- `projects.json` — add the 11 missing entries with `id`, `path` and
  `adoption`; leave the 62 existing entries unchanged.
- `scripts/workspace_check.py` — align the five rules named below; keep the
  exit-code contract (exit `1` when any ERROR is present).
- `scripts/init_project.py` — derive and enforce the canonical `id`.
- `scripts/maturity_audit.py` — delegate counts to the authoritative audit.
- `tests/` — new unittest modules and synthetic fixtures.

**Sibling files changed (data only, no source):**

- Edit `.project.json`: `campusMesh`, `careBridge`, `citylens`, `retakeKit`,
  `trailCrew`, `vowra`.
- Create `.project.json`: `dharmatlas`, `hookit`, `lileililiwen.github.io`.

**Must not change:** the `schemas/project.schema.json` property set, the
`deployment/promotions.json` ledger semantics, any `.ai-rules/*` file, and any
product source file.

## 2. Language and runtime

Python **3.11**, standard library only — matching
`release_evidence.toolchain: python-3.11` in this repo's `.project.json`. No
new third-party dependency is introduced.

Exact commands:

- `python3 -m unittest discover -s tests -v`
- `python3 scripts/workspace_check.py --self-check`
- `python3 scripts/workspace_check.py --root .. --audit`
- `python3 scripts/jenkins_manifest.py --check --root /home/paul/code`
- `node scripts/check-openspec-change-names.mjs`
- `openspec validate --changes --strict --no-interactive`

## 3. Ownership and shared code

**Project-local to `workspace-governance`.** No extraction. The repository
already owns the registry and policy (its `README.md` ownership statement);
no sibling implements registry or declaration validation, so extracting a
shared library would produce a shared owner with a single consumer.

- **Exported contract:** registry entry `{id, path, adoption}` plus
  `.project.json` schema v1 (`schemas/project.schema.json`).
- **Consumers:** `scripts/forge_governance_adapter.py`, `scripts/auto_gate.py`,
  `scripts/jenkins_manifest.py`, `scripts/maturity_audit.py`.
- **Dependency direction:** products → governance (metadata only). Governance
  never imports or depends on a product.
- **Release boundary:** versioned with this repository's `projects.json` and
  checker; consumers read, never write.

## 4. Behavioral model

**Input-state table (authoritative audit outcome):**

| Input state | Registry entry | Declaration | Audit result |
|---|---|---|---|
| dir with `.project.json`, registered, id matches | present | valid | PASS |
| dir with `.project.json`, not registered | absent | any | ERROR `DISCOVERED_UNREGISTERED` (was WARNING) |
| registered `adoption: adopted`, no declaration | present | missing | ERROR `DECLARATION_MISSING` |
| declaration `id` ≠ registry `id` | present | id differs | ERROR `DECLARATION_ID` |
| declaration not schema-conformant | present | invalid | ERROR `DECLARATION_SCHEMA` |
| adopted, valid, promoted revision ≠ `HEAD` | present | valid | WARNING `DEPLOYMENT_PROMOTION_STALE` |
| dir with no `.project.json` and no registry entry | absent | absent | no finding (not discovered) |

**Invariants:**

- At most one registry entry per directory `path`.
- At most one declaration per directory.
- `id` is unique across the registry.
- An `id` is immutable once the project has a promotion-ledger entry.

**Canonical id derivation (authoring):**

`id = kebab(dir_name)` where kebab lowercases the name and inserts `-` at
camel-case boundaries (`hestiaLab` → `hestia-lab`, `campusMesh` →
`campus-mesh`, `careBridge` → `care-bridge`, `retakeKit` → `retake-kit`,
`trailCrew` → `trail-crew`, `openTeacherKit` → `open-teacher-kit`).

When `kebab(dir_name)` is not a valid identifier — for example a dotted name
such as `lileililiwen.github.io` — the registry records an explicit alias
(`lileililiwen-site`) and `path` keeps the real directory. `init_project.py`
refuses an explicit `--id` that differs from the derived id unless the
directory name is dotted.

## 5. Contract and compatibility

- The `.project.json` schema v1 property set is unchanged; nothing is removed
  or renamed. The `id` pattern `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$` is retained,
  and the defective declarations are corrected to conform — the schema is
  **not** relaxed.
- No new audit codes are added. Existing codes are kept:
  `DISCOVERED_UNREGISTERED`, `DECLARATION_MISSING`, `DECLARATION_ID`,
  `DECLARATION_SCHEMA`, `VERIFICATION_COMMAND`, `DEPLOYMENT_PROMOTION_STALE`.
- **Level change:** `DISCOVERED_UNREGISTERED` moves WARNING → ERROR. This is a
  fail-closed tightening and is safe only because the 11 affected projects are
  registered in the same change. **Ordering:** the registry entries land before
  the level change.
- The registry gains no invented fields; only the missing `id`/`path`/`adoption`
  values are supplied. Existing entries keep their shape.

## 6. Failure and boundary policy

| Case | Result |
|---|---|
| Duplicate registry `id` | ERROR, no mutation |
| Duplicate registry `path` | ERROR, no mutation |
| Dotted directory with no recorded alias | ERROR at authoring time |
| Declaration file unparseable | ERROR (`DECLARATION_PARSE`), project skipped, audit continues |
| Empty `verification.command` | ERROR `VERIFICATION_COMMAND` |
| `deployable: true` with no ledger entry | ERROR `DEPLOYMENT_PROMOTION` (existing) |
| Adopted entry with no declaration | ERROR `DECLARATION_MISSING` |
| `maturity_audit.py` cannot read the audit | Exit non-zero with the reason; never report fewer findings than `workspace_check` |
| Zero registered projects | 0 errors, 0 warnings (not an error) |

## 7. Verification oracle

- `python3 -m unittest discover -s tests -v` — all pass, including at least one
  new case per rule above.
- `python3 scripts/workspace_check.py --root .. --audit --json` — `errors == 0`
  and zero findings whose code is in `{DECLARATION_MISSING, DECLARATION_ID,
  DECLARATION_SCHEMA, VERIFICATION_COMMAND, DISCOVERED_UNREGISTERED,
  DEPLOYMENT_PROMOTION_STALE}`. The `REQUIRED_FILE` finding for `crossalheart`
  remains and is deferred to `shared-rules-single-source`.
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings.
- `python3 scripts/jenkins_manifest.py --check --root /home/paul/code` —
  7 projects, 0 errors.
- `python3 scripts/maturity_audit.py --root .. --json` — reports the same
  `errors` and `warnings` as `workspace_check`.
- `node scripts/check-openspec-change-names.mjs` — valid;
  `openspec validate --changes --strict --no-interactive` — pass.

**Fixtures:** tests build synthetic workspaces under a temporary root covering
registered/unregistered, id-match/id-mismatch, missing-`schema_version`, and
legacy-verification-key cases. No test writes to the real workspace.

## 8. Decision ledger

**Assumptions:** the registry `id` is canonical; the directory name is the
`path` source; the existing kebab ids (`hestia-lab`, `lore-mix`,
`lileililiwen-site`) establish the kebab-with-alias convention.

**Resolved alternatives:**

- (a) Relax the schema `id` pattern vs. correct the declarations → chose
  **correct the declarations** (keeps a single identity rule).
- (b) Keep `DISCOVERED_UNREGISTERED` as WARNING vs. ERROR → chose **ERROR**,
  landed in the same change as the 11 registrations.
- (c) Recompute counts in `maturity_audit` vs. delegate → chose **delegate**
  (single source of truth).

**Deferred:** re-promotion of the 7 stale entries is a triage task, not a
blocker; if evidence refs are unavailable the warnings remain and are named.

**Blockers:** none.

**Out of scope:** the `crossalheart` `REQUIRED_FILE` error →
`shared-rules-single-source`.

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| R1 Registry completeness | register or classify every discovered declaration | §4 table, §5 ordering | `projects.json`, `workspace_check.py` | success / unregistered / not-discovered | 1.1, 2.1 | audit `DISCOVERED_UNREGISTERED` = 0 |
| R2 Declaration identity and format | one schema-valid identity per project | §4 invariants, §5 | 6 sibling `.project.json`, `workspace_check.py` | valid / id-mismatch / missing-schema / missing-declaration | 1.2, 2.2 | audit `DECLARATION_*` = 0 |
| R3 Authoring-time identity | prevent recurrence at creation | §4 derivation, §6 | `init_project.py` | derived / conflict-refused / dotted-alias | 1.3, 2.3 | new unittest cases |
| R4 Single authoritative audit | stop the two audits disagreeing | §3, §6 | `maturity_audit.py` | counts-match / audit-unreadable / empty | 2.4 | `maturity_audit` counts == `workspace_check` counts |
| R5 Promotion freshness | stale evidence cannot hide | §5, §6 | `workspace_check.py`, `promote_project.py` (invocation only) | current / stale | 2.5 | `DEPLOYMENT_PROMOTION_STALE` names both revisions |
