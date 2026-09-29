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
provider health; it should refuse to fabricate activation readiness.

This package owns one outcome: **Forge can state, read-only and
refusal-first, whether a project is a justified candidate for the
product-owned activation follow-up — and still does no billing.** It
adds a verdict, a threshold, a gate exit code and an admin-gated read
route. It adds no stored state.

## What Changes

1. A closed readiness vocabulary — `ready` | `not-ready` — and a closed
   reason vocabulary of eight named reasons, with the reason order
   fixed so output is deterministic.
2. A read-only projection over the persisted interest store returning
   one verdict per evaluated project, each carrying the window, source,
   source revision, privacy mode, coverage and freshness the verdict
   rests on.
3. A refusal-first evaluation order: absent threshold, no evidence,
   superseded-only evidence, no current window reporting the metric, a
   stale window, an inexact privacy mode, partial coverage and a value
   below the threshold each contribute their own reason. No condition
   is collapsed into a score, a percentage or a ranking.
4. An operator-declared threshold with **no default** and a bounded
   range, refused out of range rather than clamped; absence is the
   `threshold-not-declared` verdict, not an error.
5. Optional `--window` and `--source` narrowing, so a verdict can be
   scoped to a specific period or a specific analytics source.
6. A gate exit code: the report is printed (human or JSON) and the
   process exits non-zero when any evaluated project is `not-ready`,
   mirroring `forge fleet online`. Input errors remain typed refusals
   with empty stdout.
7. An admin-gated read route `GET /v1/interest/readiness` that answers
   `200` with the same verdict object in both cases and never errors on
   `not-ready`.
8. Explicit, exhaustive non-goals: no billing, subscription,
   entitlement, checkout, CRM or revenue-attribution behaviour, and no
   treatment of `paid_interest_events` as a payment record or a grant
   of access.

## Package Boundary and Split Assessment

This package owns the *gate*, not the activation. Activation is owned by
one selected product and consumes this readiness verdict as its input.
Keeping the gate in Forge and the monetization in the product is what
keeps Forge out of payment records and keeps the product in control of
its own entitlements.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `portfolio-activation-readiness` | An operator can tell, per project, whether the evidence justifies the activation follow-up | Forge, Rust/SQLite | Read-only projection over `interest-snapshots`; contract `forge-portfolio-activation/0.1.0` | `portfolio-interest-snapshots` | Readiness verdict, reason and refusal contract tests |
| Product activation | One selected product validates paid demand | selected product (not Forge) | Product-native billing and entitlements | the readiness verdict from this package | Product conversion tests |
| `standard-pack-registry-and-snapshots` | Versioned standard snapshots in generated projects | Forge, Rust/SQLite/assets | Pack descriptor and receipt | Existing profile registry, generation | Render/diff/upgrade contract tests |

The activation row is deliberately not proposed here, and portfolio-wide
billing is not proposed at all.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Interest store | `src/registry/interest/`, `interest-snapshots` spec | Append-only snapshots with privacy mode, coverage, freshness and provenance | Has no readiness verdict and no threshold vocabulary | Forge | **Extend shared owner** |
| Interest read model | `src/portfolio/interest/compare.rs`, `src/portfolio/interest_report.rs` | The refusal to sum across windows, the read-time freshness label, the closed metric allowlist | Compares projects; does not gate a decision | Forge | **Reuse as-is** |
| Interest read routes | `src/api/mod.rs` interest handlers, `authorize()` | Fleet-scoped read dispatch with `admin:access`, the `{"interest": {...}}` envelope | Has no readiness route | Forge | **Extend shared owner** |
| Gate exit-code pattern | `cmd_fleet_online` (`src/main.rs`), `cmd_gate` | Print the report on stdout, then return a typed error so the shell sees non-zero | None | Forge | **Copy the pattern exactly** |
| Product repositories | product auth, entitlements and payment stores | First-party workflow and payment data | Must never be copied into Forge | each selected product | **Adapt through a product-owned package** |
| External analytics providers | `src/analytics/`, `docs/provider-evidence.md` | Adapter contract, credential-safe evidence, not-run register | No verb emits aggregate snapshots | analytics provider | **Keep outside Forge** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Read model | New readiness projection in `src/portfolio/interest/activation.rs`; no new stored state |
| Threshold | Operator-declared, bounded `0..=1_000_000_000`; out of range is a typed refusal; absent is a verdict |
| Verdict | Closed vocabulary with one reason per withheld readiness; never a percentage or a score |
| Provenance | Every verdict carries window, source, source revision, privacy mode, coverage and freshness |
| CLI | `forge portfolio activation readiness [PROJECT] --metric <m> [--min-value <n>] [--source <s>] [--window <s>..<e>] [--stale-after-days <d>]`; report on stdout, exit non-zero when any project is `not-ready` |
| API | `GET /v1/interest/readiness` (`admin:access`), `200` with `{"interest": {"activation": …}}` for both verdicts |
| Errors | One additive variant `PortfolioActivationNotReady` → `portfolio-activation-not-ready`; bad input reuses `portfolio-interest-invalid` and `unknown-project` |
| Persistence | No new column, table or migration; one additive read-only count helper (`interest_snapshot_counts`) so `superseded-only` is distinguishable from `no-evidence` |
| Authorization | Read-only but admin-gated: readiness reveals which projects are commercially promising |
| Billing | No billing, subscription, entitlement, checkout, CRM or revenue-attribution behaviour anywhere |
| Failure | No evidence, stale window, inexact privacy mode, partial coverage and superseded-only evidence are all `not-ready` with named reasons |
| Unaffected | Public share manifest, share publication, portfolio metadata, generation, publish, journal, MCP tool list |

## Capabilities

### New Capabilities

- `portfolio-activation-readiness`: a read-only, refusal-first verdict on whether a project's aggregate interest evidence justifies the product-owned activation follow-up.

## Non-goals

- No billing, subscription, entitlement, checkout, CRM or revenue-attribution code in Forge.
- No price, plan, invoice, customer, charge or subscription type, field, route or column.
- No portfolio-wide monetization proposal: one selected product at a time.
- No automated activation, purchase, or paid-plan representation.
- No treatment of `paid_interest_events` as a payment record or a grant of access.
- No new stored state, no new table, no migration, and no change to the interest snapshot contract.
- No contact with any external provider, product or payment host.
- No portal control: the browser surface is not in this package.
- No recommendation, ranking or "best project" output; readiness is per project and evidence-only.
