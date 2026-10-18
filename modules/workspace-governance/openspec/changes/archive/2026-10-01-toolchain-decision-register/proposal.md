# Proposal: A machine-checkable register for workspace toolchain decisions

## Why

The toolchain initiative resolved four cross-repo decisions on 2026-09-30 —
agent-runtime split (D1), self-hosted identity provider (D2), capability
registry ownership (D3), crate publication mode (D4) — and they currently
live only in `TOOLCHAIN_PLAN.md` / `TOOLCHAIN_ROADMAP.md` at the workspace
root, which no script validates. Governance owns policy and declarations
(root HANDOFF ownership table), and this repository already demonstrates the
right pattern for machine-readable truth: `vocabulary.json` loaded
fail-closed through `scripts/vocabulary.py`. Decisions with downstream
change references deserve the same treatment, otherwise B3–B8 proceed on
conversational memory and audits cannot flag a decision silently rewritten.

## What Changes

- Add `decisions.json` at the governance root: versioned register of
  toolchain decisions with id, title, status lifecycle
  (`proposed|accepted|rejected|superseded`), date, owner project, decision
  text, and `downstream` references naming a registered repo plus its
  planned OpenSpec change name.
- Add `scripts/decisions.py` loader mirroring the vocabulary fail-closed
  contract, and a `workspace_check.py --decisions` summary path; seed the
  four accepted D1–D4 entries.
- Add `docs/toolchain-decisions.md` as the human view, and amend
  `docs/capabilities.md` to record the D3 ownership split (schema in
  platform-contracts, registered instances in this repo).

## Package Boundary and Split Assessment

**Single outcome:** any agent or script can query the current, auditable
workspace decisions instead of trusting prose.

**Included:** register file, loader, self-check wiring, seeded D1–D4
entries, human doc.

**Excluded (split signals):** consumption mechanisms are separate,
independently acceptable packages — the Forge governance-provider shim
exposing decisions over its stdin/stdout contract (forge change), the
workspace-governance profile updates when a decision's runtime actually
lands (e.g. the Zitadel runtime profile in B3), and per-project conformance
edits. Each one can be accepted or refused without invalidating the
register.

**Dependencies:** existing registry (`projects.json`) as the source for
`owner_project`/`downstream.repo` validation. None of the toolchain code
batches.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `toolchain-decision-register` | Decisions queryable and drift-guarded | Workspace Governance, Python 3.11 | `decisions.json` + loader + docs | projects.json registry | unittest suite + `--decisions` run |
| Shim exposure to Forge | Forge reads decisions through its provider contract | forge, Rust | `forge_governance_adapter` surface | this register | forge-side conformance change |
| Identity runtime profile | Zitadel onboarded as shared runtime | workspace-governance + jenkins-local | runtime profile + manifest | D2 entry here | B3 exit evidence |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository | `vocabulary.json`, `scripts/vocabulary.py` | fail-closed JSON loader pattern, REQUIRED_KEYS, error naming | vocabulary is enumeration, not lifecycle; no supersede concept | Workspace Governance | **extend shared owner** (sibling file, same discipline) |
| this repository | `scripts/workspace_check.py` `validate_capabilities` | registry cross-reference plumbing and finding codes | capabilities validate presence, not decisions | Workspace Governance | **adopt pattern** for owner/downstream checks |
| root docs | `TOOLCHAIN_ROADMAP.md` D1–D4 table | the decision content to transcribe | prose only, unvalidated | workspace-root index | **adopt as seed data** |
| forge | `scripts/forge_governance_adapter.py`, governance-provider-contract v0.1.0 | read-only shim precedent for cross-repo consumption | decisions are out of the current contract | forge | **defer** (separate package above) |
| platform-contracts | capability schema families | D3 assigns it the entry-schema role | schema not yet authored (B4) | platform-contracts | **keep local** policy statement only; schema work is its own repo's change |

## BFS Impact Map

- **Inputs:** four accepted decisions; registry of 77+ project ids for
  cross-reference.
- **Files:** new `decisions.json`, `scripts/decisions.py`,
  `docs/toolchain-decisions.md`; modified `scripts/workspace_check.py`
  (self-check + `--decisions`), `docs/capabilities.md` (D3 note),
  `tests/test_decisions.py`.
- **Unaffected:** profile schemas, `projects.json` content, promotion
  ledger, jenkins manifest, deployment templates, every other repository.
- **Failure/boundary:** malformed or duplicate register ⇒ loader raises
  naming the file and entry; `--decisions` output must never fall back to
  an empty list silently (vocabulary rule inherited); portfolio audit on an
  absent register is a single warning, governance self-check is an error.
- **Compatibility:** additive; consumers that do not read decisions.json
  keep working unchanged.
- **Policy restatements that MUST NOT weaken existing rules:** the D2 entry
  scopes SSO to workspace products; the root HANDOFF prohibition on shared
  login/analytics for the public GitHub Pages catalog is re-cited inside the
  entry and in `docs/toolchain-decisions.md`.

## Capabilities

- **New:** `workspace-decision-register`.

## Non-goals

- No enforcement engine, no decision voting workflow, no automatic status
  transitions; history integrity is git review plus immutability rule.
- No editing of any sibling repository to consume the register.
- No new runtime profile for Zitadel in this change (B3).
- No capability-entry schema here (platform-contracts owns it, D3).
- No changes to vocabulary.json keys or existing profiles.

## Evidence boundary

This change proves the register parses, cross-references the registry,
refuses bad states, and that the seeded entries for D1–D4 are present with
their downstream references. It cannot prove that Forge, OpenPanel, or any
product has read or honored a decision; each consumer's own change supplies
that evidence.
