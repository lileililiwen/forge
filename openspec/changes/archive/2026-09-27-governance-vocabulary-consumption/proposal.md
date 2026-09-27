# Proposal: Consume the governance vocabulary instead of re-declaring it

## Companion gate — authored, not eligible

Workspace Governance's `portfolio-vocabulary-normalization` change is authored
but not implemented: `workspace-governance/vocabulary.json` **does not exist on
this host yet** (verified by direct path lookup). This package is therefore a
proposal awaiting companion implementation evidence, exactly as
`driftwatch-cli-alignment` and `workspace-governance-adapter-consumption` were
before their siblings shipped. No part of it may be implemented before that
change lands and the file is readable.

## Why

Workspace Governance's coordination rule is explicit
(`docs/package-backlog.md`, "Coordination rules"): **"Consume the governance
vocabulary; never re-declare a field name, a state, a `kind` or a `profile`
locally."** Forge violates that rule in five measurable places, and the
violations are not cosmetic — each one is a value Forge asserts about a sibling
project without asking the sibling.

1. **Governance profile names.** `src/generate/workspace.rs` hardcodes the
   generated `.project.json` profile mapping
   (`rust-product`, `python-product`, `typescript-product`, `dotnet-product`,
   `flutter-product`; see the assertions at `:233`, `:273` and `:368`). The
   previous cycle confirmed those strings against the live sibling registry,
   which is the right instinct and the wrong mechanism: a confirmed copy is
   still a copy, and WG's change adds `vocabulary.json`
   (`{"schema_version": 1, "profiles": [...], "kinds": [...],
   "placeholder_markers": [...], "secret_field_substrings": [...]}`) precisely to
   make the copy unnecessary.
2. **`evidence_status`.** Forge emits the literal `planned` from a Rust constant
   (`src/generate/workspace.rs:40`, `const EVIDENCE_STATUS: &str = "planned"`)
   and the project's own `.project.json` was just hand-edited from `planned` to
   `implemented`. Across the 77 declarations on this host the values are
   **`implemented` (41), `verified` (33) plus one `verified-reference-only`,
   `mvp-validated` (1), and one declaration whose `evidence_status` is an entire
   prose paragraph** — no declaration still holds the template's `planned`,
   which is the value Forge emits — because
   `schemas/project.schema.json` constrains the field only by `minLength: 1` and
   WG's vocabulary change does **not** cover it. There is no source of truth to
   consume, so Forge's emission is an assertion rather than a mapping.
3. **`kind`.** This repository's own declaration says `"kind": "control-plane"`
   (`.project.json:4`). WG's remap table normalizes `control-plane` →
   `platform`, and the change makes a non-canonical declaration `kind` a
   `KIND_UNKNOWN` warning. Forge would be governed by a value it invented.
4. **Gate runtime and binary names.** `SUPPORTED_GATE_RUNTIMES`
   (`src/gate/mod.rs:82`), the probe candidates (`src/policy/mod.rs:36`) and the
   pipe-joined provider copy of the same list (`src/provider/mod.rs:84-89`,
   re-split at `:567`) each re-declare the runtime vocabulary.
5. **Secret and placeholder word lists.** `policy::redact_credentials`
   (`src/policy/mod.rs:932`) carries its own credential-word set; WG's change
   moves the authoritative copies of `secret_field_substrings` and
   `placeholder_markers` into `vocabulary.json`, and
   `platform-contracts/schemas/registry.json` already owns contract **field
   names**. Three lists, one concern — which is why the sibling change insists on
   splitting them explicitly rather than merging them.

Two further asymmetries matter for anything Forge declares about itself.

**Capabilities are declared nowhere, though the code is shipped.** Forge carries
`src/identity` (OIDC/PKCE validation, project-scoped admin sessions),
`src/release` (semver-gated stages, checks bound to a captured revision),
`src/gate` (revision-bound gate evidence) and `src/agent` (supervised runtime
delegation), yet `forge/.project.json` has **no `capabilities` block at all**
while the WG schema defines the shape (`owner`, `evidence_state`,
`evidence_ref`) and the audit reports the states. Generation emits no
`capabilities` block either — `src/generate/workspace.rs` writes `verification`
and `deployment` only — so a generated project starts as vocabulary-free as its
parent.

**The product-code quality gate is bindable but not bound.** `.ai-gate/gate.yaml`
enables `repository`, `build`, `tests` and `security` and nothing else, so
`product_code_quality.py` never runs against this repository. That is a real
finding, not a formality: measured here, `src/**/*.rs` contains **28 occurrences
of the default marker vocabulary — and zero of them are maintainer-debt markers**.
`TODO`, `FIXME`, `XXX`, `HACK`, `unimplemented!` and `todo!` return no match at
all in `src/`; every occurrence is one of the two words `placeholder` (18) and
`stub` (10), used either as legitimate domain vocabulary (`src/ui_pattern/mod.rs`
refuses "placeholder" descriptors and says so in its rejection messages) or as an
incidental identifier name (`now_placeholder()` at `src/ui_pattern/mod.rs:717` and
`src/component/mod.rs:425`, the `stub()` test helper at `src/gate/mod.rs:1070`).
Rust's lexical rule is satisfied (`#[test]`/`mod tests` are inside `#[cfg(test)]`
guards), so the only finding class is these two words. Binding the gate at
`placeholder_threshold: 0` without addressing them would be a self-inflicted red
CI; ignoring the gate leaves the portfolio's structural blind spot exactly where
it was. The honest sequence is: rename the incidental identifiers, keep the domain
vocabulary in the messages that *are* the product's contract, declare an explicit
marker override with the reason recorded, keep the debt threshold at zero so real
`TODO`/`FIXME` debt fails closed, and let the gate bind for real.

## What Changes

- **Consume `vocabulary.json` through the vendoring mechanism**
  `platform-contract-consumption` establishes: `contracts/vocabulary/`
  (governance kinds, profiles, placeholder markers) plus the registry-owned
  secret field names, each pinned in `contracts/manifest.json` with a source
  revision and sha256, verified offline.
- **Replace the copied profile mapping with consumed data.** The governance
  profile stays in the profile descriptor as *declared data*, but every value is
  validated against the consumed canonical profile set; a descriptor whose
  `governance_profile` is not canonical refuses at load with the offending value
  named, and the mapping table itself leaves `src/generate/workspace.rs`.
- **Normalize Forge's own declaration** to the canonical vocabulary: `kind` →
  `platform`, `profile` stays `rust-product` (canonical in the sibling's list),
  `schema_version: 1` unchanged, and `evidence_status` set from the agreed
  vocabulary — with a hard rule that Forge never writes a state its own evidence
  does not support, which is why the `evidence_status` vocabulary decision must
  be resolved *before* this package implements (open decision 1).
- **Declare `capabilities` honestly** for this repository: `identity`, `admin`,
  `release`, `jobs`, `storage`, `tenancy` and `billing` are *not* a wish list —
  each entry is either `configured`/`verified` with a real `evidence_ref` inside
  this checkout, or explicitly `blocked` with the non-goal recorded. Absent
  capability is stated as absent.
- **Emit `capabilities` in generated projects only when a descriptor really
  implements them** — never a copy of the parent's intent, never a speculative
  `declared` entry.
- **Add a `declaration-vocabulary` doctor finding** reporting non-canonical
  `kind`/`profile`/`evidence_status` values and `capabilities` states whose
  `evidence_ref` does not exist, `applicable: false` so it never gates health,
  mirroring how `workspace-metadata` and `gate-evidence` already behave, and
  making the checker plane emit the finding only where a project opted in.
- **Bind the product-code quality gate** with the renames and the explicit,
  evidence-recorded marker/threshold decision above, and add the check to
  `.ai-gate/gate.yaml` so the gate that Forge *drives* also runs *on* Forge.
- **Reconcile the runtime-name lists** to one consumed source shared by the
  gate, policy and provider surfaces.
- **Split secret vocabulary by role** and document it: field names in shared
  documents from the contract registry, placeholder/secret words for repository
  checks from governance `vocabulary.json`, captured-output redaction staying
  in `policy::redact_credentials`.

## BFS Impact Map

- **Capabilities:** new `governance-vocabulary-consumption`; modifies
  `deterministic-project-generation` (emitted declaration values come from
  consumed vocabulary), `doctor-maturity-assessment` (new non-gating finding),
  `gate-runtime-evidence` and `quality-policy-integration` (shared runtime-name
  source), `provider-integration-evidence` (provider row vocabulary),
  `external-checker-emission` (the new finding must project to alerts without
  changing the alert schema), and depends on
  `platform-contract-consumption` for the vendoring mechanism.
- **Users and flows:** a portfolio operator sees Forge and Forge-generated
  projects described in the workspace's own words, and can trust a Forge
  capability claim because it points at a file that exists.
- **Contracts/data/persistence:** `forge.yaml` gains no new required field;
  `.project.json` emission keeps `schema_version: 1` with consumed values; the
  ownership receipt (`src/generate/workspace.rs:33`) semantics are unchanged,
  but a vocabulary-driven content change makes previously generated
  declarations **stale-unedited**, which the existing refresh path already
  handles and this package must prove per profile.
- **Integrations/configuration:** Workspace Governance becomes a *vocabulary*
  upstream through the same opt-in, digest-pinned mechanism as contracts; no
  network, no parent-directory search, no runtime requirement when the file is
  absent — behaviour stays exactly as today with an honest
  `vocabulary-unavailable` note.
- **Callers:** `forge new`, `forge import` (which observes a declaration
  informationally today), `forge doctor`, `forge check`, `forge fleet`
  (WG-side profiles already surface verbatim and must stay verbatim), the
  portal settings/projects sections that render declaration fields, and
  `scripts/check-spec-governance.mjs`.
- **Failure/boundary behavior:** missing or unreadable `vocabulary.json` →
  consumed vocabulary reports unavailable and generation keeps its current
  values rather than guessing; a non-canonical declared value → refused by name
  at descriptor load, never coerced; a `capabilities` entry with a missing
  `evidence_ref` → the audit's own `configured`/`verified` rules already make
  that an error, so Forge must refuse to emit it; a WG registry entry whose
  profile Forge does not know → surfaces verbatim through `forge fleet` exactly
  as today, with no coercion into a Forge profile.
- **Tests:** vocabulary load and refusal paths; per-profile declaration digests
  before/after the vocabulary switch with the stale-unedited refresh proof;
  foreign-declaration preservation; receipt re-hash; doctor applicability matrix
  (declared/absent/non-canonical/unresolvable evidence ref); checker alert
  projection; fleet verbatim passthrough; the quality gate binding running
  green against this repository with its recorded residual.
- **Dependencies:** hard on WG `portfolio-vocabulary-normalization` (the file
  must exist); soft on `platform-contract-consumption` (the vendoring and digest
  mechanism); `evidence_status` normalization needs a WG decision this package
  requests but does not own.
- **Compatibility/security/privacy:** generated trees stay buildable without
  Forge and without the sibling; the vocabulary is data, never executable
  input, and is parsed with the same size bounds and rejection of traversal
  tokens as every other consumed document; no secret value ever enters a
  declaration; `capabilities` entries record paths, not credentials.

## Capabilities

- `governance-vocabulary-consumption`: Forge takes Workspace Governance's kinds,
  profiles, states and word lists as consumed data with pinned provenance,
  refuses values it cannot source, describes its own repository in that
  vocabulary with evidence-backed capability declarations, and reports
  non-canonical declarations without gating health.

## Non-goals

- Defining or renaming the portfolio vocabulary from Forge. `kind`, `profile`,
  `evidence_state` and marker lists belong to Workspace Governance; this package
  consumes them and files the `evidence_status` gap as a request with a
  dependency edge rather than inventing values here.
- Registering or adopting any project in the workspace registry (the standing
  non-goal from `workspace-metadata-emission`).
- Making the vocabulary findings gate maturity, health, the checker plane or any
  exit code.
- Retro-editing other repositories' declarations.
- Promoting Forge's maturity level, publishing anything, or claiming a gate pass.
- Replacing `policy::redact_credentials`; its captured-output role is separate
  from the vocabulary of field names.

Source: requirement.md §3.5, §7, §14, §23, §25, §32, §38, §39, §45.
