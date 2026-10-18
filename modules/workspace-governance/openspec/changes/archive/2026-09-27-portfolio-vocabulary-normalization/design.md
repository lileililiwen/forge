# Design: Normalize the portfolio vocabulary

## 1. Implementation boundary

**Repository:** `/home/paul/code/workspace-governance` — Python 3.11, standard
library only.

**Files to change:**

- `vocabulary.json` (new) — the canonical source.
- `scripts/workspace_check.py` — load the vocabulary; replace `PROFILES` and
  `SECRET_WORDS`; add `validate_kind`.
- `scripts/product_code_quality.py` — take default markers from the vocabulary.
- `scripts/maturity_audit.py` — report the vocabulary version (display only).
- `tests/test_vocabulary.py` (new).
- `projects.json` and the `.project.json` files named in §4's remap table.

**Must not change:** `platform-contracts`, `rust-platform-libs`, `driftwatchdog`
or any product source; `schemas/project.schema.json` required fields.

## 2. Language and runtime

Python **3.11**, standard library only. Commands:

- `python3 -m unittest discover -s tests -v`
- `python3 scripts/workspace_check.py --root .. --audit --json`
- `python3 scripts/workspace_check.py --self-check`
- `python3 scripts/product_code_quality.py <project_dir> --json`
- `openspec validate --all --strict --no-interactive`

## 3. Ownership and shared code

Project-local to `workspace-governance`, the owner of declaration vocabulary.

- **Exported contract:** `vocabulary.json`
  (`{"schema_version": 1, "profiles": [...], "kinds": [...],
  "placeholder_markers": [...], "secret_field_substrings": [...]}`).
- **Consumers:** `workspace_check.py`, `product_code_quality.py`,
  `maturity_audit.py`; documented for `platform-contracts`,
  `rust-platform-libs` and `driftwatchdog`, which reconcile in their own
  packages.
- **Dependency direction:** products → governance.
- **Release boundary:** ships with this repository.

## 4. Behavioral model

**Canonical profiles (15):** `governance`, `product`, `typescript-product`,
`python-product`, `rust-product`, `dotnet-product`, `typescript-monorepo`,
`dotnet-library`, `dotnet-template`, `rust-platform`, `python-runtime`,
`platform`, `tooling`, `deployment`, `inventory`.

**Canonical kinds (12):** `product`, `typescript-product`, `python-product`,
`rust-product`, `dotnet-product`, `typescript-monorepo`, `shared-library`,
`tooling`, `platform`, `governance`, `deployment`, `inventory`.

**Enforcement:**

| Input | Result |
|---|---|
| registry `profile` in the canonical set | pass |
| registry `profile` not in the set | ERROR `PROFILE_UNKNOWN` |
| declaration `kind` in the canonical set | pass |
| declaration `kind` not in the set | WARNING `KIND_UNKNOWN` |
| `vocabulary.json` missing or unparseable | each consumer exits non-zero with the reason |
| `profiles/<name>.json` naming a profile outside the set | ERROR `PROFILE_DOC_UNKNOWN` |

**Remap table (declaration `kind`):**

| Existing | Canonical |
|---|---|
| `application`, `open-source-knowledge-platform`, `open-source-assessment-platform`, `web-site` | `product` |
| `application-platform`, `runtime-platform`, `control-plane` | `platform` |
| `rust-tooling`, `runtime-operator`, `quality-gate`, `agent-runtime` | `tooling` |
| `shared` | `shared-library` |
| `infrastructure` | `deployment` |

**Remap table (declaration `profile`):** `web-platform`, `web-saas`,
`mobile-platform`, `cross-border-care-platform`, `privacy-sensitive-web-saas`
→ `product`. The implementation MUST scan every declaration for any remaining
non-canonical value and remap it rather than special-casing this list.

**Invariant:** registry `profile` drives `profile_requirements`; normalizing a
declaration `kind` or `profile` MUST NOT change any project's required-file
outcome, because requirements read the registry profile only.

## 5. Contract and compatibility

- `vocabulary.json` is new and additive; `schema_version: 1`.
- New codes: `PROFILE_UNKNOWN` (ERROR, replaces the ad-hoc allow-list path
  `PROFILE_UNKNOWN` is already emitted — the change is that it now means "not in
  the canonical set"), `KIND_UNKNOWN` (WARNING), `PROFILE_DOC_UNKNOWN` (ERROR).
- The secret list in `vocabulary.json` MUST be a superset of today's
  `SECRET_WORDS` terms and of `platform-contracts`' `secret_field_substrings`;
  a narrowing would weaken the audit and is forbidden.
- The placeholder list MUST equal today's `DEFAULT_PLACEHOLDER_MARKERS` unless a
  term is added; `quality.placeholder_markers` remains a per-project override.

## 6. Failure and boundary policy

| Case | Result |
|---|---|
| `vocabulary.json` missing | tool exits 2 with the path; no empty-list fallback (an empty secret list would silently disable the scan) |
| Unknown registry profile | ERROR `PROFILE_UNKNOWN`, names the value and the canonical set |
| Unknown declaration kind | WARNING `KIND_UNKNOWN`, names the value |
| Duplicate entry in a vocabulary list | ERROR at load; the file must be unambiguous |
| Empty list for `secret_field_substrings` or `placeholder_markers` | ERROR at load; never a pass |
| Project overrides `placeholder_markers` | still honoured; the vocabulary supplies the default only |

## 7. Verification oracle

- `python3 -m unittest discover -s tests -v` — all pass, including: canonical-set
  enforcement for both registry and declaration, remap acceptance, vocabulary
  load failure, and that the secret scan still fires with the loaded list.
- `python3 scripts/workspace_check.py --root .. --audit --json` — 0 errors; no
  `KIND_UNKNOWN` warnings; no non-canonical value remains.
- A direct scan asserts zero declarations or registry entries outside the
  canonical sets.
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings.
- `python3 scripts/maturity_audit.py --root .. --json` — counts match the audit.
- `python3 scripts/product_code_quality.py scripts --json` — unchanged PASS for
  the governance repo (markers now default from the vocabulary).
- `openspec validate --all --strict --no-interactive` — pass.

## 8. Decision ledger

**Assumptions:** kind is descriptive and must not drive requirements; profile is
already the driver; the registry's 13 in-use profiles are all canonical already
(normalized in package 1), so only declarations need remapping.

**Resolved alternatives:**

- (a) One shared `vocabulary.json` vs extending `profiles/*.json` → chose a
  single `vocabulary.json`; profile documents stay prose and are cross-checked.
- (b) ERROR on unknown kind vs WARNING → chose **WARNING**; kind is descriptive
  and a new project might legitimately introduce one before it is added.
- (c) Empty-list fallback vs hard failure on missing vocabulary → chose **hard
  failure**; an empty secret list silently disables a security check.
- (d) Move `platform-contracts`/Rust lists now vs defer → chose **defer** to
  `contracts-single-source`, to keep this change inside one owner and language.

**Deferred:** reconciling the `platform-contracts` schema registry and the Rust
redaction lists with `vocabulary.json` (`contracts-single-source`); driftwatchdog
marker ownership (`gate-run-evidence`).

**Blockers:** none.

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| R1 Single vocabulary source | one machine-readable source | §1, §3, §5 | `vocabulary.json` | loads / missing / empty | 1.2, 2.1 | tools refuse to run without it |
| R2 Canonical profiles enforced | no ad-hoc allow-list | §4, §5 | `workspace_check.py`, `projects.json` | canonical / unknown / doc mismatch | 2.2 | audit 0 errors, no `PROFILE_UNKNOWN` |
| R3 Canonical kinds enforced | descriptive vocabulary normalized | §4 | `workspace_check.py`, declarations | canonical / unknown | 2.3 | no `KIND_UNKNOWN` |
| R4 Shared word lists consumed | no duplicate lists | §5, §6 | `workspace_check.py`, `product_code_quality.py` | markers default / secret scan fires | 2.4 | tests + audit secret scan unchanged |
| R5 Portfolio normalized | zero non-canonical values | §4 remap | `projects.json`, declarations | scan clean | 3.1 | direct scan returns [] |
