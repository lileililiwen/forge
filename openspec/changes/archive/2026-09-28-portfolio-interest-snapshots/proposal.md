# Proposal: Import privacy-safe portfolio interest snapshots

## Why

Public demos and the portfolio should create evidence about which projects
deserve deeper investment. Forge needs aggregate, privacy-safe snapshots so the
owner can prioritize work without moving visitor identities or product data
into the control plane.

## What Changes

- Define an admin-only import for aggregate metrics from approved adapters.
- Store project/time-window/source/revision and metric values with provenance.
- Expose comparisons and trend summaries for prioritization.
- Keep raw events, identities, payment records, and product databases inside
  each independent product or analytics provider.

## Package Boundary and Split Assessment

This package owns only aggregate evidence and prioritization. It does not own
public links (the share package), Hugo rendering (the site package), analytics
collection, attribution identity, CRM, checkout, or paid plans. It is split
because it has a privacy/retention lifecycle and a different acceptance oracle
from public publication.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `portfolio-interest-snapshots` | Admin can compare privacy-safe project interest | Forge, Rust/SQLite | Aggregate snapshot import schema | share-publish; analytics adapter | Import and authorization tests |
| Product activation | One selected product can validate paid demand | selected product | Product-native billing/entitlements | evidence from this package | Product conversion tests |

The second row is deliberately deferred until one project has evidence; no
portfolio-wide billing is proposed.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable capability | Gap | Decision |
|---|---|---|---|---|
| Forge external analytics | `external-planes-analytics` | Provider adapters, windowed aggregates, credential-safe evidence | Needs portfolio-specific storage and redaction | **extend shared owner** |
| Individual products | product repositories and their auth/data stores | First-party workflow and payment data | Must not be copied into Forge | **adapt through a generic adapter** |
| GitHub Pages | Hugo static site | Public links and case studies | Not a private analytics backend | **keep local** |
| Platform contracts | `/home/paul/code/platform-contracts` | Cross-language fixture conventions | Snapshot schema is new | **extract recurring capability** |

## BFS Impact Map

| Area | Impact |
|---|---|
| Actor/flow | Admin imports approved aggregate evidence and compares projects |
| Data | Windowed metric snapshot, source, revision, privacy mode, provenance |
| Integrations | Optional analytics adapters; import remains valid without them |
| Security/privacy | No identity, raw event, payment, or secret data; least-privilege admin access |
| Failures | Duplicate, overlapping, malformed, stale, untrusted, or unauthorized snapshots |
| Unaffected | Public manifest, Hugo rendering, product auth, checkout, ad campaign execution |

## Capabilities

- `aggregate-snapshot-import`
- `privacy-safe-project-comparison`
- `evidence-retention-and-provenance`

## Non-goals

- No cross-project user identity graph.
- No tracking pixels or hidden collection on GitHub Pages.
- No automated project shutdown or investment decision.
- No billing, subscription, lead CRM, or revenue attribution claim.
