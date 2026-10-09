# Tasks: Portal feedback and session recovery

## Phase 1 — BFS baseline

- [x] 1.1 Confirm no active OpenSpec change and record the four audited gaps
      (silent action results, unrecoverable mid-session 401, sub-12px essential
      text, unassociated disclosure buttons) against the current file lines.
- [x] 1.2 Map every result writer and every `request`/`requestStatus` caller
      touched by the change; confirm no JSON/CLI/registry contract is affected.
- [x] 1.3 Set `current_spec: portal-feedback-session-recovery` in HANDOFF.

## Phase 2 — DFS requirement implementation

- [x] 2.1 G1: add `role="status"` to the seven `.wb-plan-result` containers in
      `frontend/index.html`.
- [x] 2.2 G1: add `setResultRole(box, isError)` in `frontend/app.js` and call
      it from every result writer (workbench plan/apply, delivery action/lookup,
      workspace bulk preview/run, scoped management preview/run, and the
      per-action preview/error/success).
- [x] 2.3 G2: add `sessionExpiredRedirect()` and wire `401` detection into
      `request` and `requestStatus`, guarded by `page !== "login"`.
- [x] 2.4 G3: append the slice-7 CSS block raising essential text to the 12px
      floor.
- [x] 2.5 G4: give each `.wb-action-body` a unique id and set `aria-controls`
      on its `.wb-action-head`.

## Phase 3 — BFS regression and completeness

- [x] 3.1 Add `tests/portal_feedback_session_contract.rs` asserting the new
      tokens and the surviving prior-slice tokens.
- [x] 3.2 Confirm `forge_web_navigation_contract` and
      `forge_web_command_catalog_contract` still pass unchanged.
- [x] 3.3 Confirm the wrong-password login path still renders its inline error
      (no redirect on `page === "login"`).

## Phase 4 — Verification

- [x] 4.1 `cargo fmt --check` and `cargo build` clean.
- [x] 4.2 `cargo test --test portal_feedback_session_contract` passes.
- [x] 4.3 `cargo test --test forge_web_navigation_contract --test
      forge_web_command_catalog_contract` passes.
- [x] 4.4 `node scripts/check-openspec-change-names.mjs` and
      `openspec validate --all --strict --no-interactive` pass.
- [x] 4.5 `git diff --check` clean and newly added files reviewed.
- [x] 4.6 Record the `forge gate --dry-run` and `forge gate` verdict in HANDOFF.
