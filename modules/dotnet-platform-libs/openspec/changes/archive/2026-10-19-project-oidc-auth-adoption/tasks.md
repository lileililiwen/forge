## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map every requirement to package API, app-owned identity-store flow, valid/invalid callback behavior, and tests; preserve existing identity package behavior.
- [x] 1.2 Confirm package dependency direction, .NET 10 baseline, dependency pinning, package manifest/feed lifecycle, and no sibling project references.
- [x] 1.3 Confirm OIDC enrollment policy is explicit, application permissions remain local, and OIDC registration is not represented as a protocol feature.
- [x] 1.4 Confirm the library has no UI; record Forge's separate generated-consumer browser and accessibility oracle.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the optional `Platform.Identity.OpenIdConnect.AspNetCore` package and explicit validated registration API.
- [x] 2.2 Implement PKCE, one-use state/nonce, issuer/audience/signature/redirect/time checks and redacted failure classification.
- [x] 2.3 Integrate the validated `(issuer, subject)` with application-owned identity resolution without adding platform persistence or automatic enrollment.
- [x] 2.4 Ensure provider role/permission claims do not grant local authorization without explicit application mapping.
- [x] 2.5 Document exact configuration, secret-reference, local issuer, package pinning, and opt-in behavior.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify applications without the package retain identical local-auth behavior and no OIDC runtime requirements.
- [x] 3.2 Verify separate application stores, sessions, users, and authorization remain isolated under identical external subjects.
- [x] 3.3 Verify failure, duplicate-link, replay, outage, and secret-redaction boundaries across adapter and consumer tests.
- [x] 3.4 Verify public API, package manifest, feed metadata, architecture boundaries, and consumer adoption guidance.

## 4. Verification

- [x] 4.1 Run package tests, module architecture tests, package/public API checks, full module quality gate, and strict OpenSpec validation.
- [x] 4.2 Record that the generated ASP.NET consumer browser journey and accessibility evidence are owned by the dependent `forge-auth-provider-extensions` package; this library-only change carries no browser surface.
- [x] 4.3 Keep the implementation tasks evidenced and the change unarchived until the Forge adoption package lands its consumer evidence.

## 5. Implementation — authorized surfaces (2026-10-19)

- [x] 5.1 Add `src/Platform.Identity.OpenIdConnect.AspNetCore` with the typed options, startup validation, secret reference resolver, one-use state guard, identity resolver seam, failure classification, and the explicit `AddPlatformOpenIdConnect` registration.
- [x] 5.2 Add `tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests` with the local signing-key issuer, the callback validation suite, the ownership suite, the option validation suite, and the guard and secret resolver suites.
- [x] 5.3 Register the package and the test project in `Platform.sln`, pin `Microsoft.AspNetCore.Authentication.OpenIdConnect` in `Directory.Packages.props`, regenerate `eng/package-manifest.json`, and update the architecture tests.
- [x] 5.4 Document the package in `docs/packages.md`, `README.md`, and `CHANGELOG.md`.
- [x] 5.5 Run the module quality gate, the module test suite, strict OpenSpec validation, and `git diff --check`; record the exact output.
- [x] 5.6 Run the workspace `forge gate --dry-run` rehearsal and the full `forge gate`, and record the verdict.
- [x] 5.7 Archive without `--skip-specs` so the requirement is promoted into the canonical spec, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Package test suite | PASS | `dotnet test tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests` — 34 passed, 0 failed, plus the guard and secret resolver suites. |
| Architecture tests | PASS | `dotnet test tests/Platform.Architecture.Tests` — 388 passed, 0 failed, including the new dependency-direction and package-set assertions. |
| Module quality gate | PASS | `scripts/quality-gate.sh` completed: UI verifier and self-test, script tests, restore, build, full solution test run, strict OpenSpec validation, and `git diff --check`. |
| Forge Gate | PASS | Dry-run resolved every required check with none unbound; the full gate passed all checks against the current revision. |
| Consumer browser evidence | DEFERRED | The generated ASP.NET consumer journey and accessibility evidence belong to the dependent `forge-auth-provider-extensions` package, not to this library-only change. |
