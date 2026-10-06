# Design: portal-login-entry-flow

## Implementation boundary

Modify the Forge Rust portal only: `src/api/ui/routes.rs` (`handle_fleet` and `handle_sign_in`), `src/api/ui/render.rs` (entry form renderer), and `tests/portal_ui_contract.rs`. Do not change identity validation, provider exchange, session storage, `/v1` handlers, or sibling projects.

## Language and runtime

Rust 2021, minimum Rust 1.87, synchronous existing `forge api serve`, Maud HTML rendering. Verify with `cargo test --test portal_ui_contract`, `cargo fmt --check`, `cargo build`, strict OpenSpec validation, name preflight, and `git diff --check`.

## Ownership and shared code

Forge owns the route and form. Reuse the existing `IdentityConfig`, `Registry`, `/ui/sign-in?project=...&return=...` OIDC start route, and Maud escaping. Platform Contracts owns cross-project wire contracts and has no relevant UI/auth contract; no sibling code is imported or modified.

## Behavioral model

| Request | Behavior |
|---|---|
| `GET /ui` with HTML and no bearer or valid session cookie | Return `303 Location: /ui/sign-in?return=%2Fui`; do not render the unauthorized error page. |
| `GET /ui/sign-in` with no `project` | Render an accessible HTML form with a required project-id field; form GET targets `/ui/sign-in`, and a hidden or fixed safe `return=/ui` is used. |
| `GET /ui/sign-in?project=<id>&return=<path>` | Keep existing validation and provider redirect unchanged. |
| Form submit for an unknown or identity-unconfigured project | Existing safe typed refusal; no challenge/session is written. |
| Authenticated `GET /ui` | Keep current fleet render behavior. |

The chooser deliberately asks for a project id instead of listing registered projects, so unauthenticated use does not reveal the registry. Use Maud's escaping and validate the return path through the existing `safe_return_path` logic. Do not place credentials in form fields or URLs.

## Contract and compatibility

Only anonymous HTML `GET /ui` changes from `401` HTML to `303` local redirect. Non-HTML requests retain existing API routing behavior. `/ui/sign-in` without a project changes from `400 ui-sign-in-missing-project` to a `200` HTML form; project-specific sign-in remains a `303` provider redirect. Authenticated `/ui`, `/v1` JSON envelopes, bearer headers, cookies, and OIDC callback contracts are unchanged.

## Failure and boundary policy

- Missing/invalid cookie on `/ui`: redirect to local sign-in entry, never echo cookie values.
- Missing project on sign-in: render form, not an auth failure.
- Invalid project, missing identity configuration, or provider setup error: preserve the existing typed safe HTML error; create no challenge/session.
- External or malformed return path: preserve existing `/ui` fallback.
- Non-HTML and JSON routes: preserve existing behavior.

## Verification oracle

In `tests/portal_ui_contract.rs`, assert an unauthenticated HTML `/ui` response has status 303 and exact local sign-in location; assert `/ui/sign-in` renders a required project input and form action; assert escaped values cannot inject markup; retain tests that project-specific start redirects to the provider and persists a challenge. Run `cargo test --test portal_ui_contract`, then the named format/build/OpenSpec/diff checks above.

## Decision ledger

- Resolved: show a project-id form rather than a registry listing, to avoid unauthenticated inventory disclosure.
- Resolved: redirect only browser HTML navigation; leave API negotiation and JSON behavior unchanged.
- Resolved: reuse the already implemented OIDC flow; do not mint sessions from form input.
- No blocking decisions remain.
