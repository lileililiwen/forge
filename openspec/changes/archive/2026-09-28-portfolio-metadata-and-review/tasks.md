# Tasks: portfolio-metadata-and-review

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map existing registry migrations, project identity, portal view models/routes, provider observations, and active `portal-web-ui` dependency.
      Mapped: `Registry::open` runs `SCHEMA_SQL` then `apply_migrations` then
      `reconcile_journal`; `projects`/`operations` are the only shared tables and
      stay byte-identical. Project identity is `validate_project_id` +
      `Registry::inspect`. The portal view models are `src/api/ui/{data,render,routes}.rs`
      and are extended in place. Provider observations
      (`gate/evidence.rs`, `governance.rs`, `publish/providers.rs`) stay
      source-owned and are only *imported*.
- [x] 1.2 Freeze tables, enum values, relation constraints, snapshot freshness, authorization, migration rollback, and CLI/API/portal fixtures.
      Eight additive tables in `src/registry/portfolio.rs`; lifecycle
      (`incubating|building|validating|operational|paused|archived`), confidence
      (`unknown|low|medium|high`), relation types
      (`depends-on|duplicate-of|shares-domain-with|replaces|consumes|optional-provider`)
      and evidence states (`observed|stale|unavailable|invalid|not-run`) are closed
      enums in `src/portfolio/mod.rs`. Freshness is a `stale_after` bound that can
      only downgrade `observed` to `stale`. Authorization re-uses
      `api::authorize` for JSON and the existing bearer/`Origin`/form-token
      checks for the browser. Rollback is one explicit
      `BEGIN`/`COMMIT`/`ROLLBACK` batch.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add additive SQLite schema and transactional migration/rollback behavior.
      `PORTFOLIO_SCHEMA_SQL` is applied by `apply_portfolio_migration` inside one
      explicit `BEGIN IMMEDIATE` … `COMMIT`, with `ROLLBACK` on any failure, so an
      interrupted create leaves neither a partial table nor a half-applied index.
      Every statement is `IF NOT EXISTS`; `migration_preserves_a_pre_change_registry`
      opens a hand-built pre-change registry and proves the project and journal
      rows survive and the new domain works on the migrated file.
- [x] 2.2 Implement project lifecycle, confidence, tags, goals, relations, reviews, blockers, and next actions.
      `Registry::{portfolio_write, portfolio_record, portfolio_add_tag,
      portfolio_remove_tag, portfolio_tags_for, portfolio_all_tags,
      portfolio_add_relation, portfolio_remove_relation, portfolio_relations_for,
      portfolio_all_relations, portfolio_record_review, portfolio_reviews_for,
      portfolio_add_goal, portfolio_link_goal, portfolio_goal, portfolio_goals}`.
      An absent field stays `None` rather than defaulting: an unclassified
      project never reads as `incubating`.
- [x] 2.3 Implement redacted provider/evidence snapshot import with source revision and staleness semantics.
      `Registry::portfolio_import_snapshot` requires source system, source
      revision, RFC 3339 observation time and status, bounds the payload to
      64 KiB, requires a JSON *object*, refuses a bound preceding the
      observation, and passes the payload through
      `policy::redact_credentials` before writing. There is no update or
      delete path: `portfolio_current_snapshots` picks the newest row per
      source and `portfolio::effective_status` downgrades an expired
      `observed` to `stale` while preserving the stored status verbatim.
- [x] 2.4 Add Core/CLI/API contracts and extend the existing portal list/detail/filter projections.
      CLI: `forge portfolio tag|relation|review|goal|evidence|show`. JSON API:
      `GET /v1/projects/{id}/portfolio` (read) plus
      `POST …/portfolio/{tags,relations,reviews,evidence}` behind the existing
      `authorize()`. Browser: `GET /ui?tag=&lifecycle=&confidence=` gains a
      filter form and three portfolio columns, `GET /ui/projects/{id}` gains the
      portfolio projection and its write form, and
      `POST /ui/projects/{id}/portfolio` is the only browser write path.
- [x] 2.5 Add domain, migration, import, API, and portal contract tests.
      17 `src/portfolio` + 20 `src/registry::portfolio` unit tests, 23
      `tests/portfolio_contract.rs` CLI/API tests and 20
      `tests/portfolio_ui_contract.rs` browser tests.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove existing registry, inventory, provider, portal, CLI, and JSON contracts remain compatible.
      `existing_registry_contracts_stay_compatible` asserts the `forge list`
      project row shape is unchanged and carries no portfolio field;
      `the_existing_ui_routes_still_answer_without_portfolio_state` proves
      `/ui`, the detail page and the publish plan preview are unchanged; the
      full workspace suite runs to completion with 72 result groups and 0
      failures, and `/healthz`, `/v1/projects` and `Accept: application/json`
      on `/ui` answer byte-identically in the live smoke.
- [x] 3.2 Exercise duplicate, invalid, unavailable, stale, unauthorized, rollback, and partial-provider cases across all callers.
      Duplicate tag (one link), duplicate relation (one row), self relation
      (refused), unknown project (typed not-found, no state), out-of-vocabulary
      values, malformed/non-object evidence, expired bound, unavailable
      provider, unauthorized JSON mutation (401), cross-project session (403),
      missing/foreign `Origin` and form-token mismatch (403) on the browser
      write, plus
      `an_interrupted_migration_rolls_back_and_keeps_the_prior_registry_usable`,
      which fails the batch mid-way through a decoy table and proves no partial
      portfolio table survives and the next open retries the whole batch.

## 4. Verification

- [x] 4.1 Run format, build, clippy, focused/full tests, strict OpenSpec validation, and `git diff --check`.
      `cargo fmt --all -- --check` PASS for every touched file (the pre-change
      baseline drift in `src/gate/evidence.rs`, `src/publish/{fleet,jenkins}.rs`
      and the gate/publish test suites is untouched — verified by reverting
      `cargo fmt`'s edits to those files); `cargo build` PASS;
      `cargo clippy --all-targets -- -D warnings` reports the same 10
      pre-existing locations as the stashed baseline and zero new ones;
      `cargo test --workspace --all-targets -- --skip
      rust_scaffold_builds_and_tests_with_native_toolchain`: 72 groups, 0
      failed; `node scripts/check-openspec-change-names.mjs` PASS;
      `openspec validate --all --strict --no-interactive`: 50 passed, 0 failed;
      `cargo deny check`: advisories/bans/licenses/sources ok; `git diff
      --check` PASS.
- [x] 4.2 Record SQLite migration evidence separately from provider/runtime evidence; do not claim remote or multi-user readiness.
      Migration evidence is local SQLite only: the schema is additive, the
      batch is transactional, and a hand-built pre-change registry migrates
      forward with its rows intact. No PostgreSQL server, multi-user
      collaboration, SSO, remote synchronization or automatic prioritization
      is claimed. Provider evidence is fixture-only: no external provider is
      contacted, no gate/gate-runtime pass is claimed, and every imported
      snapshot is labelled with its own source system and revision so Forge
      never claims to have performed the check.
