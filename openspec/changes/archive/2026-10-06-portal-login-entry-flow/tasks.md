## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map unauthenticated, authenticated, malformed-project, and JSON route outcomes to existing portal tests and caller contracts.
- [x] 1.2 Confirm the form uses the existing OIDC start route and does not enumerate projects or accept credentials.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add failing route-contract coverage for `/ui` redirect and the missing-project sign-in form.
- [x] 2.2 Implement the local redirect and accessible escaped project-id form while preserving project-specific OIDC behavior.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify OIDC start/callback tests, authenticated portal rendering, return-path refusal, and `/v1` contracts remain valid.
- [x] 3.2 Review unauthenticated information disclosure, HTML escaping, and credential handling.

## 4. Verification

- [x] 4.1 Run portal contract tests, formatting, and build; record exact results.
- [x] 4.2 Run OpenSpec name preflight, strict validation, and `git diff --check`; review all added files.
- [x] 4.3 Confirm every scoped scenario has local evidence and archive through the default spec-promotion path; update HANDOFF after archive.
