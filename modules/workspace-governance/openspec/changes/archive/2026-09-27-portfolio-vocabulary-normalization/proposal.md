# Proposal: Normalize the portfolio vocabulary and single-source the shared word lists

## Why

Two vocabularies drifted without a source of truth.

**Kind and profile.** The 77 declarations carry **22 distinct `kind` values and
17 distinct `profile` values**, several of them one-off prose
(`cross-border-care-platform`, `privacy-sensitive-web-saas`, `web-platform`,
`mobile-platform`, `open-source-knowledge-platform`, `runtime-operator`,
`quality-gate`, `control-plane`). The checker admits profiles through an ad-hoc
allow-list plus suffix rules (`workspace_check.validate_registry`), so a
descriptive value silently passes or fails depending on spelling, and
`profiles/*.json` describes only 6 of the 17 in use.

**Shared word lists.** The placeholder markers and secret-field substrings that
decide whether a check fires are declared in more than one place:
`scripts/product_code_quality.py:DEFAULT_PLACEHOLDER_MARKERS`,
`scripts/workspace_check.py:SECRET_WORDS`,
`platform-contracts/schemas/registry.json:secret_field_substrings`, and
`rust-platform-libs/crates/platform-testing/src/reasons.rs`. Driftwatchdog
documents the same placeholder vocabulary for its `placeholder-threshold`
concern without owning a copy. A word added in one place does not reach the
others.

Both are governance-owned vocabulary, so this is the same owner, lifecycle and
oracle as the two shipped packages.

## What Changes

- Add `vocabulary.json` at the governance root as the single machine-readable
  source for the canonical `kinds`, canonical `profiles`, `placeholder_markers`
  and `secret_field_substrings`.
- Replace the ad-hoc profile allow-list in `workspace_check.py` with the
  canonical profile set: a non-canonical registry `profile` is ERROR
  `PROFILE_UNKNOWN`, and a non-canonical declaration `kind` is WARNING
  `KIND_UNKNOWN`.
- Have `workspace_check.py` read `secret_field_substrings` and
  `product_code_quality.py` read `placeholder_markers` from `vocabulary.json`
  instead of their local copies.
- Normalize the declarations and registry entries that use non-canonical
  values, using the remap table in the design.
- Add a check that `profiles/*.json` and `vocabulary.json` agree, so the two
  governance-side profile views cannot drift again.

## Package Boundary and Split Assessment

**Single outcome:** one canonical vocabulary source, enforced, with every
existing declaration normalized to it.

**Included:** `vocabulary.json` (new), `profiles/*.json`, `schemas/project.schema.json`
(description only), `scripts/workspace_check.py`,
`scripts/product_code_quality.py`, `scripts/maturity_audit.py` (report only),
`tests/`, and the declarations/registry entries named in the remap table.

**Excluded:** `platform-contracts` and `rust-platform-libs` copies of the secret
list (they consume the contract; changing them is `contracts-single-source`);
any driftwatchdog source change (`gate-run-evidence`); any product behavior.

**Split signals considered:** the kind/profile set and the shared word lists are
both governance vocabulary edited by the same owner with one oracle
(`workspace_check --audit` clean plus zero non-canonical values). Splitting them
would produce two near-identical changes touching the same two scripts.

**Dependencies:** `registry-and-declaration-integrity` (shipped) for the registry
and the enforced identity rules; `shared-rules-single-source` (shipped) for the
non-destructive report pattern this change reuses.

**Package map:** package 3 of 8. Package `gate-run-evidence` depends on it;
`governance-front-door` references it.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| workspace-governance `profiles/` | `profiles/product.json`, `profiles/inventory.json`, … (6 files) | profile documentation and `supported_capabilities` | covers 6 of 17 profiles in use; no machine-readable vocabulary | workspace-governance | **extend shared owner** |
| `scripts/workspace_check.py` | `PROFILES`, `SECRET_WORDS`, `validate_capabilities` | the enforcement hook and finding vocabulary | allow-list is ad-hoc; secret list is local | workspace-governance | **extend shared owner** |
| `scripts/product_code_quality.py` | `DEFAULT_PLACEHOLDER_MARKERS`, `SECRET_WORDS` | placeholder checker | duplicate list; overridable per project | workspace-governance | **extend shared owner** |
| platform-contracts | `schemas/registry.json:secret_field_substrings` | language-neutral schema registry | same concept, different encoding; consumed by Rust/DotNet | platform-contracts | **keep local here**; reconcile in `contracts-single-source` |
| rust-platform-libs | `crates/platform-testing/src/reasons.rs`, `platform-audit/src/context.rs` | redaction reasons | re-derives the platform-contracts list | rust-platform-libs | **keep local**; reconcile in `contracts-single-source` |
| driftwatchdog | `src/gate/concerns.rs` (placeholder vocabulary) | gate concern IDs | documents the marker list without owning a copy | driftwatchdog | **keep local**; reconcile in `gate-run-evidence` |
| product repositories | `.project.json` `kind`/`profile` | declarations | 22 kinds / 17 profiles in use | each product | **adopt** the canonical set |

No sibling owns a portfolio vocabulary source, so no extraction is authorized;
the canonical owner is `workspace-governance`, and the other repos consume it
through their own packages.

## BFS Impact Map

- **Capabilities:** kind/profile vocabulary; placeholder and secret word lists.
- **Consumers:** `workspace_check.py`, `product_code_quality.py`,
  `maturity_audit.py`, `forge_governance_adapter.py` (observation payload), and
  every `.project.json`.
- **Contracts:** `vocabulary.json` shape; the canonical kind/profile sets; the
  marker and secret lists.
- **Failure and boundary:** non-canonical registry profile → ERROR; non-canonical
  declaration kind → WARNING; missing `vocabulary.json` → the tools exit
  non-zero with the reason (never a silent empty list, which would make the
  secret scan vacuous).
- **Tests:** vocabulary load, canonical-set enforcement, remap acceptance, and a
  test that the secret scan still fires with the loaded list.
- **Dependencies:** Python 3.11 standard library only.
- **Compatibility, privacy, security:** kind normalization is descriptive and
  cannot change profile requirements; profile normalization must preserve each
  project's existing required-file outcome; the secret list must stay a
  superset of today's, or the audit would weaken.

## Capabilities

- `portfolio-vocabulary`.

## Non-goals

- No change to `platform-contracts`, `rust-platform-libs` or driftwatchdog.
- Not removing `kind` or `profile` from the schema; both stay required.
- Not making `kind` drive requirements (only `profile` does).
- Not reclassifying any project's lifecycle or deployment state.
