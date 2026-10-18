## 1. BFS — Baseline and impact coverage

- [x] T1 Map route dispatch, project registration/remote parsing, registry transactions, Gate evidence path, CLI command tree, standard snapshots, and Quality checks/browser harness to each requirement and scenario.
- [x] T2 Add contract fixtures for signed OIDC claims, old registry migration, exact and conflicting replay, unbound repository, and empty/failed hosted history.
- [x] T3 Confirm the producer workflow and Forge UI form the same end-to-end observation lifecycle; record exact API, uploader, schema and browser contracts.

## 2. DFS — Requirement-by-requirement implementation

- [x] T4 Implement RS256 GitHub OIDC verification with fixed issuer/audience, bounded JWKS cache/fetch and closed required claims.
- [x] T5 Add immutable GitHub repository-ID bind operation and additive registry migration; implement append-only idempotent evidence insert/read and concurrent replay conflict behavior.
- [x] T6 Add public OIDC ingest route and admin-session bind/history routes with bounded redacted errors and project scope.
- [x] T7 Add a pinned standard uploader script to construct the existing envelope from real Gate evidence and GitHub Actions runtime identity, request ID token and upload with bounded retries.
- [x] T8 Add versioned service/client standard snapshots with least-privilege upload job; preserve existing snapshots and update defaults explicitly.
- [x] T9 Extend the project Quality checks card with immutable repository binding and hosted evidence history, including loading/empty/error/stale/success states.

## 3. BFS — Cross-surface regression and completeness

- [x] T10 Verify signed identity and envelope claims are cross-checked; invalid binding, claim, size and replay paths do not write observations; uploader output never includes tokens.
- [x] T11 Verify local Gate status and delivery preconditions remain unchanged by hosted observations; hosted result is never presented as local approval.
- [x] T12 Verify concurrent inserts, additive old-registry initialization, route authorization/origin, generated workflow permissions, uploader audience/timeout and missing configuration behavior.
- [x] T13 Run the signed-in browser flow including 390px layout, keyboard/focus, accessible status, empty and stale history states, and unchanged local Gate status.

## 4. Verification

- [x] T14 Run focused Rust/API/registry/workflow/uploader tests, changed-file formatting and JavaScript syntax/browser/accessibility checks; record exact commands/results.
- [x] T15 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, `git diff --check`, `forge gate --dry-run`, and bounded full `forge gate`; attribute pre-existing failures and ensure no new failures.
- [x] T16 Revisit the BFS impact map, update every task checkbox only with evidence, archive without `--skip-specs`, commit implementation/archive, update HANDOFF pointer/evidence and create HANDOFF-only commit.
