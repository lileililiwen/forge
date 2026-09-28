# Design: portfolio-activation-readiness

## Implementation boundary

Repository: `forge`, Rust Core/CLI, existing SQLite registry. Inspect
`src/portfolio/interest/`, `src/portfolio/interest_report.rs`,
`src/registry/interest/`, CLI dispatch in `src/main.rs`, and the API
dispatch in `src/api/mod.rs`. Add a read-only projection and its
verdict vocabulary. Do not modify the interest snapshot schema, the
share or publication packages, generation, publish, or any product
repository. Do not add a payment, subscription or entitlement type.

## Language and runtime

Rust 1.87+ using the existing Cargo workspace and SQLite strategy.
Primary commands are `cargo fmt --check`, `cargo build`, focused
`cargo test`, full `cargo test --all-targets` with the repository's
native-scaffold skip where needed, and strict OpenSpec validation.

## Ownership and shared code

Forge owns the readiness verdict, its threshold vocabulary and its
refusal reasons. A selected product owns entitlement, pricing, checkout
and conversion measurement. The analytics provider owns collection and
identity policy. This package adds no adapter, contacts no provider and
persists nothing: it reads the snapshots another package already
imported and answers one question about them.

## Behavioral model

Readiness is a closed verdict, never a score:

```
readiness = ready | not-ready
reason    = no-evidence | no-current-window | stale-window
          | inexact-privacy-mode | partial-coverage | superseded-only
          | below-threshold | threshold-not-declared
```

A project is `ready` only when **all** of the following hold, and each
failed condition contributes its own reason rather than being collapsed
into one:

1. It has at least one **current** (`accepted`) snapshot.
2. Every snapshot the verdict rests on has `coverage == complete`.
3. Every such snapshot has `privacy_mode == exact-count`. A
   `lower-bound` figure is a floor and an `undeclared` figure is not
   known to be exact; neither may be read as a headcount.
4. The window is **not** stale against the operator's staleness bound.
5. The metric under evaluation meets the operator's **declared**
   threshold for the declared window.

The threshold has no default. Readiness without a declared threshold is
`not-ready` with `threshold-not-declared`, because a default would be
Forge inventing a commercial judgement it does not own. An
out-of-range threshold is a typed refusal, never a clamp.

## Contract and compatibility

Planned CLI contract:

```text
forge portfolio activation readiness [PROJECT] \
    --metric <metric> --min-value <count> \
    [--stale-after-days <days>] [--window <start>..<end>]
```

Output is a verdict per project with its reasons and the provenance the
verdict rests on, mirroring the interest projections: `window_start`,
`window_end`, `source`, `source_revision`, `privacy_mode`, `coverage`,
`freshness`. A later JSON API route and the portal consume the same
Core projection; neither is in this package.

The interest snapshot contract is unchanged. This package adds no
column, no table and no migration, so a registry written by
`portfolio-interest-snapshots` is read as-is and a registry written
before this package behaves identically.

## Failure and boundary policy

| Case | Result |
|---|---|
| No snapshots for the project | `not-ready`, reason `no-evidence`; never a zero |
| Only superseded revisions | `not-ready`, reason `superseded-only` |
| All current windows stale | `not-ready`, reason `stale-window` |
| `privacy_mode` is `lower-bound` or `undeclared` | `not-ready`, reason `inexact-privacy-mode` |
| `coverage` is `partial` | `not-ready`, reason `partial-coverage` |
| Below the declared threshold | `not-ready`, reason `below-threshold` |
| No threshold declared | `not-ready`, reason `threshold-not-declared` |
| Threshold out of range | typed `portfolio-interest-invalid`, nothing reported as ready |
| Unknown project | typed `unknown-project` |
| Unknown or unlisted metric | typed `portfolio-interest-invalid` naming the allowlist |
| Any caller expecting a plan, price or entitlement | absent by construction; Forge exposes no such field |

## Verification oracle

Unit tests for the verdict vocabulary, each reason in isolation, the
combination order, the threshold bound and the refusal to default it,
and freshness interaction with the staleness bound. CLI contract tests
for `ready`, every `not-ready` reason, an unknown project, an unknown
metric and an out-of-range threshold, plus a read-only assertion that
the registry bytes and journal row count are unchanged by a readiness
run. Cross-surface tests that the share manifest, the portfolio
projection and the fleet list gain no readiness or billing field.

Provider and product evidence is explicitly out of scope: no analytics
provider is contacted and no product is activated. Run
format/build/clippy/tests, the name preflight, strict OpenSpec
validation and `git diff --check`.

## Decision ledger

- Forge owns the gate; the product owns the monetization. Moving either
  across that line is a separate, product-owned proposal.
- `paid_interest_events` remains an aggregate signal. It is not a
  payment record, it is not a customer identity, and it grants nothing.
- Readiness is decision support, not a promise of business outcome; it
  reports evidence sufficiency, never expected revenue.
- Absence of evidence is `not-ready`, never an optimistic default. A
  project nobody measured is not a project nobody wanted.
- There is no unresolved design blocker. What the follow-up needs is a
  product and a reviewed verdict; this package supplies only the
  verdict half and is deliberately inert without it.
