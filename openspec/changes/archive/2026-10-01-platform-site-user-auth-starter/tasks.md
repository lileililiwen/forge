# Tasks: platform-site-user-auth-starter

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map existing template options, package pins, Identity lifecycle contracts, authorization policies, test-only fakes, consumer adoption tests, and app-owned persistence rules.
- [x] 1.2 Add generated-output fixtures for `EnableSiteUsers=false/true` and baseline route/permission cases before implementation.
- [x] 1.3 Confirm `.NET 10` package versions and detached-app behavior; prove existing `EnableIdentity` semantics remain unchanged.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `EnableSiteUsers` template option and conditional generated package references/files.
- [x] 2.2 Render app-owned Identity user/context, cookie registration, initial migration, login/logout/recovery pages, and starter layout.
- [x] 2.3 Render owner bootstrap command with random one-time password and a Production refusal.
- [ ] 2.4 Render opt-in self-registration and app-owned permission catalog/policies with deny-by-default behavior.
  (Corrected by `platform-shared-auth-pages`: this task was recorded complete, but no
  self-registration page was rendered — only the permission catalog and policies were.
  The app-owned permission catalog and policies are done; the page arrived later, in
  `openspec/changes/platform-shared-auth-pages`.)
- [x] 2.5 Add test-environment fake authentication and document incremental adoption/rollback.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify disabled template output has no new packages, routes, auth tables, or startup behavior.
- [x] 3.2 Verify generated app login/recovery/lockout, cookie/CSRF, permission allow/deny, and environment failure boundaries.
- [x] 3.3 Prove the generated app builds/tests independently and production packages never reference `Platform.Identity.Testing`.

## 4. Verification

- [x] 4.1 Run focused template tests, `./scripts/conformance.sh`, `./scripts/quality-gate.sh`, `./scripts/check-public-api.sh`, `git diff --check`, and strict OpenSpec validation.
- [x] 4.2 Record generated-project verification separately from any consuming product adoption; no consumer migration is in scope.
