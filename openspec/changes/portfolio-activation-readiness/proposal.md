# Proposal: activation readiness gate for a selected product

## Why

`portfolio-interest-snapshots` deliberately stopped short of product
activation: it stores aggregate demand signal and refuses to be a
billing surface. Its package-boundary table names the next row —
"Product activation: one selected product can validate paid demand" —
and defers it "until one project has evidence; no portfolio-wide
billing is proposed". The design decision ledger repeats it: Forge
stores `paid_interest_events` as an aggregate signal, "not a payment
record and cannot authorize access", and product-native monetization is
"a follow-up package for one selected product".

That gate is currently prose. Nothing in Forge can answer the question
the gate depends on — *does any project have reviewed aggregate
evidence sufficient to justify handing activation to a product?* — and
nothing refuses the move when the answer is no. An operator can read
the interest store, do arithmetic by hand across windows, and start
billing work with no evidence at all. Forge already refuses to fabricate
provider health; it should refuse to fabricate activation readiness the
same way.

This package owns one outcome: Forge can state, read-only and
refusal-first, whether a project is a justified candidate for the
product-owned activation follow-up — and still does no billing.

## What Changes

- Add a read-only activation-readiness projection over the persisted
  interest store: per project, `ready` or `not-ready` with every reason
  that produced the verdict, plus the window, source, source revision,
  privacy mode, coverage and freshness the verdict rests on.
- Make the verdict refusal-first: stale windows, `lower-bound` or
  `undeclared` privacy modes, `partial` coverage, superseded-only
  evidence, and a project with no evidence all yield `not-ready` with a
  named reason. Forge never rounds a floor up to a headcount, never
  reads a collection gap as "nobody was interested", and never invents a
  zero.
- Require an explicit threshold the operator declares, rather than a
  readiness rule Forge assumes on the operator's behalf, and refuse an
  out-of-range threshold rather than clamping it.
- Record the handoff boundary: when a project is ready, Forge names the
  **selected product** as the owner of the next package and takes no
  action itself. Forge adds no billing, subscription, entitlement,
  checkout, CRM or revenue-attribution behaviour, and
  `paid_interest_events` continues to grant nothing.

## Package Boundary and Split Assessment

This package owns the *gate*, not the activation. Activation is owned by
one selected product and consumes this readiness verdict as its input.
Keeping the gate in Forge and the monetization in the product is what
keeps Forge out of payment records and keeps the product in control of
its own entitlements.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `portfolio-activation-readiness` | Operator can tell whether a project justifies the activation follow-up | Forge, Rust/SQLite | Read-only readiness projection over `interest-snapshots` | `portfolio-interest-snapshots` | Readiness verdict and refusal contract tests |
| Product activation | One selected product validates paid demand | selected product (not Forge) | Product-native billing and entitlements | readiness verdict from this package | Product conversion tests |
| `standard-pack-registry-and-snapshots` | Versioned standard snapshots in generated projects | Forge, Rust/SQLite/assets | Pack descriptor and receipt | Existing profile registry, generation | Render/diff/upgrade contract tests |

The second row is deliberately not proposed here and is not proposed as
portfolio-wide billing in any case.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Interest store | `src/registry/interest/`, `interest-snapshots` spec | Append-only snapshots with privacy mode, coverage, freshness and provenance | Has no readiness verdict and no threshold vocabulary | Forge | **Extend shared owner** |
| Interest read model | `src/portfolio/interest/compare.rs`, `src/portfolio/interest_report.rs` | The refusal to sum across windows and the read-time freshness label | Compares projects; does not gate a decision | Forge | **Reuse as-is** |
| Product repositories | product auth, entitlements and payment stores | First-party workflow and payment data | Must never be copied into Forge | each selected product | **Adapt through a product-owned package** |
| External analytics providers | `src/analytics/`, `docs/provider-evidence.md` | Adapter contract, credential-safe evidence, not-run register | No verb emits aggregate snapshots | analytics provider | **Keep outside Forge** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Read model | New readiness projection; no new stored state, no schema change |
| Threshold | Operator-declared, bounded, validated; an out-of-range value is a typed refusal |
| Verdict | Closed vocabulary with a reason per withheld readiness; never a percentage or a score |
| Provenance | Every verdict carries window, source, source revision, privacy mode, coverage and freshness |
| CLI | `forge portfolio activation readiness [PROJECT]`, read-only |
| Persistence | None: the projection reads the existing interest tables and writes nothing |
| Authorization | Read-only, but admin-gated like every interest route: readiness reveals which projects are commercially promising |
| Billing | No billing, subscription, entitlement, checkout, CRM or revenue-attribution behaviour anywhere |
| Failure | No evidence, stale window, inexact privacy mode, partial coverage and superseded-only evidence are all `not-ready` with named reasons |
| Unaffected | Public manifest, share publication, portfolio metadata, generation, publish and journal |

## Capabilities

### New Capabilities

- `portfolio-activation-readiness`: a read-only, refusal-first verdict on whether a project justifies the product-owned activation follow-up.

## Non-goals

- No billing, subscription, entitlement, checkout, CRM or revenue-attribution code in Forge.
- No portfolio-wide monetization proposal: one selected product at a time.
- No automated activation, purchase, or paid-plan representation.
- No treatment of `paid_interest_events` as a payment record or a grant of access.
- No new stored state, no new table, and no change to the interest snapshot contract.
- No external provider contact: this package reads what a provider already imported.
