# Design: Governance vocabulary consumption

## Ownership and boundaries

| Concern | Owner | This package's role |
| --- | --- | --- |
| Canonical `kind`, `profile`, `evidence_state`, placeholder markers, secret field names; `vocabulary.json` itself | Workspace Governance | Consumer. Reads, validates against, refuses values it cannot source. Never renames, never adds |
| Contract field-name vocabulary and status enums for shared documents | `platform-contracts` | Already consumed through `platform-contract-consumption`; this package keeps the two lists separate by role |
| Declaration *shape* (`schema_version: 1`, required blocks) | Workspace Governance `schemas/project.schema.json` | Forge keeps emitting a schema-valid declaration; only the *values* become consumed rather than copied |
| Maturity level of any project | evidence per requirement.md §25 | Unchanged. A canonical vocabulary does not imply maturity |
| Registry membership, adoption, promotion | Workspace Governance scripts | Not touched. Forge continues to emit declarations without registering or adopting anything |

Consumed and declared vocabulary are different objects and the design keeps them
apart: Forge **declares** its own `.project.json`; it **consumes** the word lists
that say what a declaration value may be. The consumed file never becomes
executable input, and it never relocates a path.

## Consumption layout

```text
contracts/vocabulary/
  governance-vocabulary.json          # verbatim copy of vocabulary.json
  secret-field-substrings.json        # from platform-contracts registry.json
contracts/manifest.json               # source revisions + per-file sha256 (shared)
```

- Two sources, one manifest, each file carrying its own `source` and `revision`.
  `contracts/vocabulary/README.md` records which list governs which concern.
- Resolution for reads at runtime: an explicit `--vocabulary PATH`, then
  `FORGE_GOVERNANCE_VOCABULARY`, then the vendored file. A path that exists but
  fails to parse is a typed refusal, never a silent fallthrough to the vendored
  copy — the same rule `FORGE_GATE_BIN` already follows for the gate plane.
- If the vendored file is absent or does not match its manifest digest, Forge
  behaves exactly as it does today with its current values and reports
  `vocabulary-unavailable` in the surfaces that consulted it. **Absence of the
  vocabulary never blocks generation, import or doctor.** This keeps the
  "standalone by default" invariant intact.
- `scripts/sync-contracts.mjs` (from `platform-contract-consumption`) gains the
  governance source; a sync commit changes only `contracts/vocabulary/` plus the
  manifest.

## The `evidence_status` problem, stated plainly

Measured on this host today across the 77 `*/.project.json` declarations:

| value | count |
| --- | --- |
| `implemented` | 41 |
| `verified` | 33 |
| `verified-reference-only` | 1 |
| `mvp-validated` | 1 |
| a multi-clause prose paragraph | 1 |
| `planned` (the template value Forge emits) | 0 |

`schemas/project.schema.json` bounds `verification.evidence_status` only by
`minLength: 1`, and `portfolio-vocabulary-normalization` canonicalizes
`profiles` and `kinds` but **not** `evidence_status`. So there is nothing to
consume for this field yet, and the spellings in use are semantically different
claims about the same project — `implemented` and `verified` in particular are
41 and 33+1 declarations asserting materially different things with no
definition distinguishing them.

Design position for this package:

- Forge keeps emitting `planned` for generated projects, because `planned` is
  the value Workspace Governance's own template
  (`templates/project/.project.json:10`) uses and because a freshly generated
  project has intent but no observed evidence.
- Forge does **not** adopt `implemented` or `verified` for itself except through
  the vocabulary WG chooses, and never from a hand edit. The pending local edit
  of this repository's own declaration from `planned` to `implemented` is
  therefore *not* evidence: `verified` requires a native evidence artefact, and
  `implemented` is not defined anywhere. It stays out of this package's commits
  until the vocabulary defines it, and the package records the request upstream.
- The `declaration-vocabulary` finding treats an undefined `evidence_status`
  value as non-canonical and reports it, without inventing a canonical set.
- If WG canonicalizes `evidence_status` into `vocabulary.json`, the finding
  becomes a real comparison and this package's follow-up maps Forge's emissions
  to it. This is recorded as an open dependency, not silently assumed.

## Capability declarations for this repository

Each entry is decided against what exists in this checkout, not against the
product vision. `evidence_ref` is a repository-relative path that must exist;
`configured` and `verified` states make the audit check that, so a claim with a
dead pointer is an audit error, not a documentation nicety.

| Capability | State | Basis |
| --- | --- | --- |
| `identity` | `configured` | `src/identity` implements PKCE/S256 challenge, callback/claims validation, project-scoped sessions and revocation with real contract tests; no live OIDC issuer round trip is recorded, so `verified` is not available |
| `admin` | `configured` | admin-claim allow-listing separated from provider login (`identity-permission-denied`) plus the API's `admin:access` requirement |
| `jobs` | `configured` | `src/agent` supervised sessions and `src/procedure` bounded workflows, journalled with real verdicts |
| `storage` | `configured` | `src/registry` SQLite registry with append-only journal and idempotency columns |
| `release` | `configured` | `src/release` semver, revision-bound checks and stage outcomes; **not** `verified`, because publication remains `not-run` |
| `quality-gate` | `configured` | `src/gate` revision-bound evidence, `src/checker` emission, both exercised live against the sibling; the gate plane's own pass remains unclaimed for this repo |
| `tenancy` | `blocked` | Forge has no tenant model; AGENTS.md and requirement.md §44 make it a non-goal, so `blocked` is the honest state |
| `billing` | `blocked` | the planner's explicit billing prohibition |

Project-defined names (`quality-gate` above) are allowed by the WG schema's
`propertyNames` pattern, but they draw `CAPABILITY_UNSUPPORTED` whenever the
governance profile spec does not list them. Today `workspace-governance/profiles/`
contains no `rust-product.json`, so `load_profile_spec` returns `None` and the
supported-capability check silently cannot fire
(`workspace_check.py:161-173`, `:463-471`). The design assumption is that WG's
vocabulary change will ship profile specs for the canonical profiles (it adds a
"profiles and vocabulary agree" check). If `profiles/rust-product.json` appears
without `quality-gate` in `supported_capabilities`, Forge's declaration will draw
a warning that must be resolved by WG or by dropping the name — and this package
records that as a companion request rather than pre-empting it. Until then the
declaration states the standard names only, plus `quality-gate` explicitly
flagged in the change's evidence as pending a profile decision.

## Generation: consumed values, unchanged receipts

- The governance profile moves from the hardcoded mapping into the profile
  descriptor's own field, validated against the consumed canonical profile set
  at load. A descriptor naming a non-canonical value refuses at load with the
  field and value named — the same shape as the existing blank-`governance_profile`
  refusal.
- `capabilities` in generated declarations are emitted **only** when the profile
  descriptor declares them. No speculative `declared` entries, no copying of
  Forge's own capability set: a generated `rust-web` project does not provide
  identity just because the control plane does.
- Consequence to prove, not assume: changing declaration *content* makes a
  previously generated declaration **stale-unedited**. The existing refresh path
  must handle it — refresh the declaration, re-hash the receipt, name both files
  in `files_changed`, and leave a user-edited declaration alone under the
  standard ownership-conflict refusal. Verification runs the per-profile
  digests and the `--no-workspace-metadata` byte-identity check again, plus an
  upgrade round trip on a project generated before the switch.
- A foreign `.project.json` (written by `init_project.py` or a human, no Forge
  receipt) stays foreign: never rewritten, never blocked on.

## The `declaration-vocabulary` doctor finding

- Applicability: only when a `.project.json` exists and a vocabulary is
  consultable. Otherwise `applicable: false`, exactly like `workspace-metadata`
  and `gate-evidence`, so absence never reads as health and the checker plane
  stays quiet for projects that never opted in.
- Reports: non-canonical `kind`, non-canonical `profile`, `evidence_status`
  outside the consumed set (or vocabulary unavailable, stated as such),
  `capabilities` entries whose `evidence_ref` is missing or absolute or
  traversing, and `deployable: true` with no `release_evidence`.
- Severity: `warn` for vocabulary divergence, `fail` only for a *Forge-authored*
  claim that cannot be honoured (an evidence ref Forge emitted and that no
  longer resolves). Never lowers maturity, never changes an exit code, never
  affects `forge check` for projects that did not opt in.
- Redaction and char bounds are applied to every finding string like everywhere
  else, and the assessed project's absolute path becomes `<project>`.

## Binding the product-code quality gate

Measured today, `src/**/*.rs` holds **28 occurrences** of the default marker
vocabulary and satisfies Rust's lexical test rule (every `#[test]` and
`mod tests` sits behind a `#[cfg(test)]` guard). The split matters more than the
total: **zero** occurrences are maintainer-debt markers (`TODO`, `FIXME`, `XXX`,
`HACK`, `unimplemented!`, `todo!` all return no match in `src/`), and all 28 are
the two words `placeholder` (18) and `stub` (10), which in this codebase are
legitimate vocabulary:

- **Incidental identifiers** that should be renamed because the word is not the
  point: `now_placeholder()` (`src/ui_pattern/mod.rs:717`,
  `src/component/mod.rs:425`) and the `stub()` test helper
  (`src/gate/mod.rs:1070`). Renaming these to intent-revealing names that do not
  contain a marker word removes them from the count on merit.
- **Domain vocabulary** that is the product's contract: the UI-pattern and
  component registries refuse "placeholder" descriptors and say so in their
  rejection messages (`src/ui_pattern/mod.rs:18`, `:160`, `:460`, `:533`,
  `:1489`), and the deploy state comments name `placeholder` as the shape of an
  unrecorded observation (`src/deploy/mod.rs:646`).

Sequence, in this order, so the gate binds for real rather than nominally:

1. Rename the incidental identifiers.
2. Declare `quality.product_roots: ["src"]`, `quality.languages: ["rust"]`, and
   an explicit `placeholder_markers` override that keeps `TODO`, `FIXME`, `XXX`,
   `HACK`, `unimplemented!`, `todo!` and **drops the two words that are Forge
   domain vocabulary** (`placeholder`, `stub`) — with the reason written in the
   `.ai-rules/completion.md` placeholder paragraph, not hidden in a JSON field.
3. Keep `placeholder_threshold: 0` against that reduced set, so genuine debt
   fails closed.
4. Bind `python3 scripts/product_code_quality.py <dir>` into
   `.ai-gate/gate.yaml` as a real check.
5. Record the pre-change count (28), the post-change count, and the residual
   matches in `docs/release-readiness.md` so the decision is auditable.

This is a narrowing of scope, not a waiver: the gate still fails on maintainer
debt, and the override is stated as a vocabulary decision with a reason rather
than a raised threshold. The alternative — a threshold of 28 — would pass the
check while proving nothing, and is explicitly rejected here.

## Compatibility and migration

- No Forge runtime contract, exit code, journal row or `forge.yaml` key changes.
- `.project.json` for this repository changes `kind` (`control-plane` →
  `platform`) and gains `capabilities` + `quality`. Both are additive or
  vocabulary-driven; `workspace_check.py` stays at zero errors, which is the
  portfolio oracle WG requires after any package.
- Generated declarations change content, which the receipt/refresh path already
  models. Nothing about `forge new`'s file set changes except the optional
  `capabilities` block for descriptors that declare it.
- `forge fleet` keeps surfacing WG profiles verbatim; consumption must not
  coerce a foreign profile into a Forge profile, and the existing verbatim test
  stays as the guard.
- `scripts/check-spec-governance.mjs` is Forge's own status-claim checker and is
  *not* the governance vocabulary; it keeps its local rules and this package
  adds nothing to its word lists.

## Open decisions (recorded, not silently fixed)

1. **`evidence_status` has no source of truth.** This package requests WG fold
   it into `vocabulary.json`; until then Forge emits the template value and
   reports divergence without adjudicating it. A hand-edited value in this
   repository's own declaration is not evidence and is not committed.
2. **`CAPABILITY_UNSUPPORTED` depends on a profile spec that does not exist
   yet.** If WG ships `profiles/rust-product.json`, Forge's declared names must
   be inside its `supported_capabilities` list or the warning is correct and the
   declaration shrinks.
3. **Whether Forge's own `kind` is `platform` or a governance-owned new name.**
   The remap table says `control-plane` → `platform`. If WG decides a distinct
   `control-plane` kind is worth canonicalizing, that is WG's call and Forge
   follows it; this package does not lobby through implementation.
4. **Marker override versus renaming.** Renaming is preferred where the word is
   incidental; the override exists only for words that are the product contract.
   A reviewer should disagree here rather than discover it later.

## Verification

- `python3 scripts/workspace_check.py --project forge --root ..` and
  `python3 scripts/product_code_quality.py forge` run against this repository,
  with their output recorded verbatim in the change's evidence — including the
  exact pre/post placeholder counts and the `--json` summary shape.
- Vocabulary tests: load, refuse-on-unparseable, absent-vendored-file fallback
  behaviour, explicit-path-beats-env, non-canonical value refusal, and the
  `vocabulary-unavailable` reporting path.
- Generation matrix: per-profile declaration digests before/after, receipt
  re-hash, stale-unedited refresh, edited-declaration conflict, foreign
  declaration untouched, `--no-workspace-metadata` byte-identity, and native
  build/test of a generated tree with the changed declaration present (the
  declaration stays inert metadata).
- Doctor/checker/portal/fleet cross-surface tests for the new finding's
  applicability and its non-gating guarantee.
- `openspec validate --all --strict --no-interactive`, the Node preflight
  checks, `git diff --check`, and the standing Rust suite.
