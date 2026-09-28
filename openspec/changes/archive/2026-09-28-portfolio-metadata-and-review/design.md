# Design: portfolio-metadata-and-review

## Implementation boundary

Repository: `forge`, Rust Core/CLI/API, existing SQLite registry, and the
active `portal-web-ui` surface. Add a portfolio domain/module, additive
migrations, query/mutation contracts, and portal projections. Inspect/change
`src/registry/`, `src/portal/`, `src/api/`, migration wiring, and contract
tests. Do not alter provider-owned evidence semantics or existing JSON
contracts except additively.

## Language and runtime

Rust 1.87+, existing SQLite, serde JSON, and the existing server-rendered HTML
portal. Use the repository's current Cargo format/build/clippy/test commands;
no new database server or frontend framework.

## Ownership and shared code

Forge owns user portfolio metadata and the local aggregate read model. External
systems own their observations. Existing provider contracts are reused; no
shared portfolio library is extracted because the persistence and UI lifecycle
are Forge-specific.

## Persistence model

Additive tables:

```text
portfolio_projects(project_id, lifecycle, confidence, next_action, blocker, reviewed_at)
portfolio_tags(id, name, color, created_at)
portfolio_project_tags(project_id, tag_id)
portfolio_relations(id, from_project, to_project, relation_type, note)
portfolio_goals(id, title, status, description)
portfolio_goal_projects(goal_id, project_id)
portfolio_reviews(id, project_id, confidence, note, reviewed_at)
portfolio_evidence_snapshots(id, project_id, source_system, source_revision,
  observed_at, status, stale_after, evidence_json)
```

`project_id` references Forge's canonical imported identity. Tags and relations
are user-owned. Evidence snapshots are append-only. A current read model picks
the newest valid snapshot, but preserves stale/unavailable states.

Allowed lifecycle values are `incubating`, `building`, `validating`,
`operational`, `paused`, and `archived`. Confidence values are `unknown`,
`low`, `medium`, and `high`.

Relation types are `depends-on`, `duplicate-of`, `shares-domain-with`,
`replaces`, `consumes`, and `optional-provider`.

## Contract and compatibility

Core operations:

```text
portfolio tag add|remove|list
portfolio relation add|remove|list
portfolio review set
portfolio goal add|link|list
portfolio evidence import|list
```

Every mutation is project-scoped, validated, idempotent where an identity is
provided, and recorded with an operation identity. Existing registry/project
commands remain compatible. Provider evidence imports require source system,
source revision, observation time, status, and redacted evidence payload.

## Behavioral model

External snapshot states are `observed`, `stale`, `unavailable`, `invalid`, or
`not-run`. A missing provider is `unavailable`, not healthy. User metadata is
independent of external state: a project may be high priority while its gate is
failed or unavailable.

Portal list filtering combines user-owned fields and imported read-only fields.
Edits to tags/reviews do not mutate repository files or provider records.

## Failure and boundary policy

| Case | Result |
|---|---|
| Duplicate tag name | typed conflict, no duplicate row |
| Self relation | refused unless relation type explicitly permits it; current types do not |
| Duplicate relation | idempotent no-op or existing relation returned |
| Unknown project | typed not-found, no mutation |
| Unknown provider/source | unavailable snapshot with explicit source, never guessed pass |
| Malformed evidence | invalid snapshot rejected or stored as invalid with reason; never healthy |
| Migration interruption | SQLite transaction rolls back; prior registry remains usable |
| Unauthorized web mutation | existing API authorization refusal; no state change |

## Verification oracle

Add migration round-trip and rollback tests, domain tests for lifecycle/confidence
and relation constraints, CLI/API contract tests, provider-import fixtures,
stale/unavailable projection tests, and portal tests for filters, labels,
confidence, blocker, next action, and evidence status. Verify old registry
commands against pre-migration fixtures. Run format/build/clippy/tests, strict
OpenSpec validation, and `git diff --check`; browser capture is supplemental,
not the only UI evidence.

## Decision ledger

- SQLite is the v1 persistence boundary; PostgreSQL is deferred until a
  demonstrated multi-user or remote synchronization requirement exists.
- Forge stores snapshots, not authoritative external evidence.
- Tags are free-form names with validation; a future vocabulary can be layered
  on without changing project identity.
- The package depends on the existing portal package and must not duplicate its
  route/auth architecture.
