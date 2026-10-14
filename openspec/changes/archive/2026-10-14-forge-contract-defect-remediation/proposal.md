# Proposal: forge-contract-defect-remediation

## Why

The manager heavy-check pass over the six portal quick-win changes first
executed the portal contract-test suite end to end. It surfaced **8 contract
defects** in subagent-delivered handler / frontend / test code (recorded in
`HANDOFF.md` under *Known defects — contract tests*). They are not regressions
from the manager integration commit `21a3967` (behavior-preserving renames +
formatting); they were simply never run until the manager pass, because the
subagents only ran lightweight checks.

The features are functionally complete but **not production-verified**: the
contract envelope for several reads is wrong, two test expectations contradict
the established wire contract, and three static-asset assertions anchor on
markup the server / frontend does not actually emit. This change closes the
gap so the six delivered changes can be considered verified.

## What Changes

Eight targeted corrections, grouped by where the defect lives:

### Handler defects (Rust — `src/`)
1. **Identity rotation revocation order** (`src/api/admin/identity.rs`):
   the handler calls `revoke_all_sessions` the moment the *current* password
   verifies, *before* the new-password strength check. A weak new password with
   a correct current password therefore logs the operator out (session count →
   0) and then returns `400`. Fix: capture the probe session token returned by
   `authenticate` and revoke **only that probe token** on the failure path; the
   Core `change_password` already revokes all sessions on success. On a weak
   password the operator's session is preserved.
2. **Portfolio interest managed-project guard** (`src/api/admin/interest.rs`):
   the three read handlers (`interest_list`, `interest_show`,
   `handle_interest_trend`) open the registry and call Core without confirming
   `id` is a managed project, so an unmanaged id falls through to a generic
   `400`. Fix: add a `require_managed(db_path, id)` guard that returns
   `404` with code `portfolio-unmanaged-project`, `effect: "none"`, and no
   echoed id (matching the contract the other admin reads already honor).

### Frontend defects (`frontend/`)
3. **Interest card container id** (`frontend/index.html`): the portfolio
   interest `wb-card` carries `aria-labelledby="portfolio-interest-title"` but
   no `id="portfolio-interest"`; the wiring test expects the container id.
   Fix: add `id="portfolio-interest"` to the card section (no behavior change).

### Contract-test defects (test-only corrections; the server/frontend are
correct, the test expectations were wrong)
4. **Router `admin_routes_are_anonymous_401`**: sending `GET` to the
   `POST`-only `route_identity` is correctly answered `405 method-not-allowed`
   by `src/api/router.rs`. Fix the test to exercise the session gate with an
   anonymous `POST` + `application/json` body (which the handler answers `401
   api-unauthorized`), preserving the original intent (every admin variant is
   session-gated).
5. **Router `unknown_project_id_is_a_typed_400_through_admin_dispatch`**: the
   canonical, established envelope for an unmanaged project id is
   `404 admin-project-unmanaged` (emitted by `deploy_id_gate` and relied on by
   seven other tests). The test expected a non-existent `400
   admin-project-not-managed`. Fix the test to expect `404
   admin-project-unmanaged` (still a typed envelope, so the dispatch
   classification stays load-bearing).
6. **Portfolio `interest_reads_are_read_only_and_touch_no_rows`**: the trend
   path is passed with `?metric=…&limit=…` embedded in the URL, but the test
   `request` helper sets `request.query = None`; the handler reads
   `request.query`, so the metric is never parsed and the read returns `400`.
   The production server populates `request.query` from the URL (server.rs:579),
   so this is purely a test-harness gap. Fix: drive the trend path through
   `get_query` (which sets `request.query`).
7. **Gate `workbench_health_card_carries_gate_subrow_with_rerun_button`**: the
   test anchors the health-card block with `html.find("wb-health-title")`, which
   lands *inside* the `aria-labelledby="wb-health-title"` attribute and slices
   off the `aria-labelledby="` prefix, so the block can never contain the full
   attribute. Fix the anchor to `aria-labelledby="wb-health-title"` so the
   existing correct markup satisfies the assertion.
8. **A11y `reduced_motion_block_covers_rail_panels_cards`**: the reduced-motion
   block already names `.lifecycle-rail`, `.project-panel`, `.wb-card` and
   `transition:none!important` — but the test's "not an empty rule" guard
   asserts `!css.contains("prefers-reduced-motion: reduce){")`, which a valid
   non-empty block *does* contain. Fix the test to assert the empty-rule form
   `prefers-reduced-motion: reduce){}` is absent.

## Impact

- No public route is added or removed; no CLI surface changes.
- The two handler fixes change only error envelopes and session-revocation
  scope — verified read paths are unchanged.
- Six contract test files are corrected; the corrected expectations match the
  established wire contract (so they will not conflict with the seven tests that
  already rely on `404 admin-project-unmanaged`).
- `frontend/index.html` gains one id attribute; no script or behavior change.
- After this change all eight defects are resolved and the portal contract suite
  is green, so the six quick-win changes become production-verified.
