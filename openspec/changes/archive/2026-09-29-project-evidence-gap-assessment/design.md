# Design: project-evidence-gap-assessment

Status: implemented and archived 2026-09-29.

## Implementation boundary

Repository `forge`, Rust 1.87+, existing doctor/evidence stack.

Files added:

- `src/doctor/gaps.rs` — the catalog-gap rule set and finding projection.
- `tests/project_gaps_contract.rs`, `tests/project_gaps_cross_surface.rs`.

Files changed additively:

| File | Change |
|---|---|
| `src/doctor/mod.rs` | register the catalog-gap inspector beside the existing inspectors |
| `src/main.rs` | `Project` command gains `gaps [PROJECT]` with `--format table\|json\|ndjson` and `--category` / `--status` / `--remediation-class` filters |
| `src/catalog/mod.rs` | consumed the catalog record as the gap input through the existing public catalog module (depends on `project-catalog-query-contract`) |

Untouched by this change: file writers, GitHub transport, semantic generation,
CI execution and deployment. This package is read-only.

## Language and runtime

Rust 1.87+, `rustfmt` defaults, no new dependency. Commands mirror the doctor
cycle: `cargo test --lib -- doctor::gaps`, `cargo test --test
project_gaps_contract`, `cargo clippy --all-targets -- -D warnings`.

## Ownership and shared code

Forge owns finding production. The package **extends** `src/doctor/mod.rs`
reusing its verdict vocabulary (`PASS`, `WARN`, `FAIL`, `UNAVAILABLE`,
`NOT_APPLICABLE`), stable finding IDs and evidence attribution; it **adopts**
`src/spec/mod.rs` remediation classes (`automatic|semantic|manual`) rather than
inventing a second routing table. Driftwatchdog and workspace-governance remain
optional providers reached only through their existing adapters.

## Behavioral model

A finding is an observation, never a stored verdict promoted to health.

| Field | Rule |
|---|---|
| `id` | stable, derived from `(rule, project_id, subject)` |
| `status` | one of the closed doctor verdicts |
| `category` | `description \| tags \| ci \| compose \| manifest \| docs \| repository` |
| `remediation_class` | `automatic \| semantic \| manual` |
| `evidence` | source, source revision, observed_at, freshness — redacted |

Rules detect **missing** (no value), **stale** (freshness `stale`),
**invalid** (value fails its own contract), **unavailable** (source unreadable)
and **unverified** (value present but no corroborating evidence). A gap that is
not applicable to a project's profile reports `NOT_APPLICABLE`, never `FAIL`.

## Contract and compatibility

`forge-project-evidence/0.1.0`. JSON/NDJSON findings are the contract; the table
is human-facing. Errors: `catalog-invalid` for a malformed filter,
`unknown-project` for a project target. No new error type is required. Scope is
single-project or whole-catalog; the fleet form evaluates every catalog record
in `(project_id, source)` order.

## Failure and boundary policy

| Case | Result |
|---|---|
| Project has no metadata at all | findings for each applicable category; never one collapsed health value |
| Source unavailable | `UNAVAILABLE` finding naming the source; not `FAIL`, not omitted |
| Metadata not applicable to profile | `NOT_APPLICABLE`, excluded from any "healthy" count |
| Provider body contains a secret | redacted before the finding is rendered |
| Two sources disagree | both findings retained, each attributed, no merge |

## Verification oracle

`tests/project_gaps_contract.rs`: fixture catalog producing one finding per
category and each verdict, stable IDs, filter composition, JSON/NDJSON parity;
`tests/project_gaps_cross_surface.rs`: read-only proof, credential redaction,
`UNAVAILABLE`/`NOT_APPLICABLE` boundaries, and that doctor's existing findings
are unchanged. No checkbox without its named test and captured output.

## Decision ledger

- Missing metadata is not a deployment failure unless the profile makes it
  applicable; applicability is profile-derived, not guessed.
- Findings never promote health and never trigger a write; repair is the
  separate `project-local-remediation-plans` package.
- No production readiness is inferred from repository metadata alone.
