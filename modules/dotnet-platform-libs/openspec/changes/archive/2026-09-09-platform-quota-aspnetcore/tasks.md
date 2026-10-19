## 1. Adapter boundary

- [x] 1.1 Create `Platform.Quota.AspNetCore` referencing `Platform.Quota` and ASP.NET Core only.
- [x] 1.2 Define resource/subject resolvers, exemption metadata, unavailable-provider policy, and response options.
- [x] 1.3 Adapt `QuotaEnforcementMiddleware.cs` and its tests from the starter kit without copying plan or Finbuckle types.

## 2. Enforcement implementation

- [x] 2.1 Implement middleware/filter ordering guidance and check/reserve/settle integration.
- [x] 2.2 Implement 429 ProblemDetails, `Retry-After`, correlation, trace, and safe diagnostic extensions.
- [x] 2.3 Implement configurable missing-context and provider-unavailable policies.
- [x] 2.4 Implement endpoint/method/health exemptions and idempotent DI registration.

## 3. Verification and documentation

- [x] 3.1 Add tests for allowed, denied, reset timing, missing context, provider unavailable, cancellation, and exemptions.
- [x] 3.2 Add architecture tests keeping core quota packages free of ASP.NET Core.
- [x] 3.3 Document migration from the starter middleware and ownership of limits/entitlements.
- [x] 3.4 Run serial tests, `git diff --check`, and strict OpenSpec validation.
