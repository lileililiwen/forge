# Design: forge-contract-defect-remediation

All eight corrections are local and low-risk. They do not add routes, change
the CLI, or alter any read path that already works. Each fix is documented with
its precise location and the invariant it restores.

## 1. Identity rotation revocation order (`src/api/admin/identity.rs`)

`change_password_inner` (around line 225) currently does:

```rust
match authenticate(db_path, &email, &body.current) {
    Ok(Some(_)) => { revoke_all_sessions(db_path)?; }   // <-- logs everyone out
    ...
}
change_password(db_path, &body.password)?;              // also revokes all on success
```

`authenticate` mints a session and returns `Ok(Some(token))`. The probe token is
the only session the rotation attempt should consume on failure. Replace the
`revoke_all_sessions` call with `revoke(db_path, &token)`, revoking **only the
probe token**. `crate::identity::global::change_password` already calls
`revoke_all_sessions_in` on success, so the success path is unchanged and the
failure path no longer destroys the operator's session. Verified by the existing
test `weak_password_is_refused_and_does_not_mutate_the_registry`, which expects
the active session count to be unchanged after a weak-password attempt.

## 2. Portfolio interest managed-project guard (`src/api/admin/interest.rs`)

Add a module-private helper:

```rust
fn require_managed(db_path: &Path, id: &str) -> Result<(), ApiResponse> {
    let managed = Registry::open(db_path)
        .ok()
        .and_then(|r| r.inspect(id).ok())
        .is_some();
    if managed { Ok(()) } else {
        Err(ApiResponse::json(404, json!({
            "error": { "code": "portfolio-unmanaged-project",
                       "message": "this project is not managed by this Forge registry; register it from a terminal first, then try again." },
            "effect": "none",
            "contract": INTEREST_CONTRACT_VERSION,
        })))
    }
}
```

Call it at the top of the `guarded` closure in `interest_list`,
`interest_show` and `handle_interest_trend`. The session gate (401) still runs
first; an unmanaged-but-authenticated id now returns the typed `404
portfolio-unmanaged-project` with `effect: "none"` and no echoed id, matching
the contract test `an_unmanaged_id_is_refused_and_changes_no_state`.

## 3. Interest card container id (`frontend/index.html`)

The portfolio interest `wb-card` (around line 540) reads
`<section class="wb-card" aria-labelledby="portfolio-interest-title">`. Add the
container id so the wiring test can target it:

```html
<section class="wb-card" id="portfolio-interest" aria-labelledby="portfolio-interest-title">
```

No script or style change; only the static id is added.

## 4–8. Contract-test corrections (server / frontend are correct)

These fix the test harness, not the implementation:

- **4 router anonymous 401**: `route_identity` is `POST`-only
  (`src/api/admin/identity.rs:38`); a `GET` is correctly `405`. Change the test
  to send an anonymous `POST` with `application/json` (still gated to `401
  api-unauthorized`), preserving the "every admin variant is session-gated"
  intent.
- **5 router unknown project**: the canonical envelope is `404
  admin-project-unmanaged` (from `deploy_id_gate`, used by seven other tests).
  The test expected the non-existent `400 admin-project-not-managed`. Change the
  expectation to `404` + `admin-project-unmanaged`.
- **6 interest read-only trend**: drive the trend path through `get_query` so
  `request.query` is populated (the production server does this at
  server.rs:579); the embedded `?metric=…` in the path is otherwise ignored by
  the `request` helper.
- **7 gate health card anchor**: anchor the block with
  `html.find("aria-labelledby=\"wb-health-title\"")` so the slice retains the
  full attribute (the current anchor lands mid-attribute and slices off the
  prefix).
- **8 a11y reduced-motion empty rule**: assert the *empty* rule is absent —
  `!css.contains("prefers-reduced-motion: reduce){}")` — instead of asserting
  the opening brace of a valid non-empty block is absent.

## Verification

- `cargo test --test web_portal_identity_rotation_contract`
- `cargo test --test web_gate_controls_contract`
- `cargo test --test router_table_driven_contract`
- `cargo test --test web_portfolio_interest_contract`
- `cargo test --test web_portal_responsive_a11y_contract`
- Full `cargo test --workspace --all-targets`, `cargo clippy`, `cargo fmt --check`.
- `openspec validate --all --strict --no-interactive`.
