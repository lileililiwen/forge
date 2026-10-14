# Tasks: forge-contract-defect-remediation

## 1. Identity rotation revocation order
- [ ] In `src/api/admin/identity.rs`, capture the probe token from `authenticate` and revoke only that token (replace `revoke_all_sessions(db_path)` with `revoke(db_path, &token)`).

## 2. Portfolio interest managed-project guard
- [ ] Add `require_managed(db_path, id)` to `src/api/admin/interest.rs` returning `404 portfolio-unmanaged-project` with `effect: "none"` and no echoed id.
- [ ] Call `require_managed` at the top of the `guarded` closure in `interest_list`, `interest_show`, `handle_interest_trend`.

## 3. Interest card container id
- [ ] Add `id="portfolio-interest"` to the portfolio interest `wb-card` in `frontend/index.html`.

## 4. Router anonymous-401 test
- [ ] In `tests/router_table_driven_contract.rs`, replace the `GET` to the `POST`-only change-password route with an anonymous `POST` + `application/json` and assert `401 api-unauthorized`.

## 5. Router unknown-project test
- [ ] In `tests/router_table_driven_contract.rs`, change `unknown_project_id_is_a_typed_400_through_admin_dispatch` to expect `404` + `admin-project-unmanaged`.

## 6. Interest read-only trend test
- [ ] In `tests/web_portfolio_interest_contract.rs`, drive the trend path through `get_query` so `request.query` is populated.

## 7. Gate health-card anchor test
- [ ] In `tests/web_gate_controls_contract.rs`, anchor the health-card block with `aria-labelledby="wb-health-title"` so the slice retains the full attribute.

## 8. A11y reduced-motion empty-rule test
- [ ] In `tests/web_portal_responsive_a11y_contract.rs`, assert the empty reduced-motion rule `prefers-reduced-motion: reduce){}` is absent.

## 9. Verify and archive
- [ ] Run the five affected contract-test binaries; confirm all green.
- [ ] Run `cargo test --workspace --all-targets`, `cargo clippy`, `cargo fmt --check`.
- [ ] Run `openspec validate --all --strict --no-interactive`.
- [ ] Commit OpenSpec package + fixes; archive the change; update HANDOFF evidence.
