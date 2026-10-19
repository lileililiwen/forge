# Platform.Identity lifecycle

Provider-neutral identity lifecycle contracts and ASP.NET Core integration seams.
The platform never owns users, roles, claims, password hashes, signing keys, or
migrations; it orchestrates lifecycle operations behind stable, safe-failure
contracts that applications adapt to their identity store.

## Packages

| Package | Purpose |
| --- | --- |
| `Platform.Identity.Contracts` | Framework-neutral lifecycle contracts (refresh, password recovery, two-factor, impersonation), stable outcome codes, and the `IIdentityLifecycleCoordinator` composition. |
| `Platform.Identity.AspNetCore` | ASP.NET Core integration: `AddPlatformIdentityLifecycle` and minimal-API mappers for refresh, password recovery, two-factor, and impersonation endpoints. |
| `Platform.Identity.Testing` | Deterministic, non-production fakes: `InMemoryRefreshTokenStore`, `FakePasswordRecoveryService`, `FakeTwoFactorService`, `FakeImpersonationService`, `DenyAllImpersonationPolicy`, `AllowImpersonationPolicy`, `RecordingIdentityAuditHook`. |
| `Platform.Identity.EntityFrameworkCore` | Unchanged. The platform does not own identity persistence; the application supplies its own `IIdentityStore` adapter. |

## Adoption

```csharp
// 1. Register the application-owned stores and policies.
services.AddSingleton<IRefreshTokenStore, MyHashedRefreshTokenStore>();
services.AddSingleton<IPasswordRecoveryService, MyPasswordRecoveryService>();
services.AddSingleton<ITwoFactorService, MyTwoFactorService>();
services.AddSingleton<IImpersonationPolicy, MyImpersonationPolicy>();
services.AddSingleton<IImpersonationService, MyImpersonationService>();
services.AddSingleton<IIdentityAuditHook, MyAuditHook>();

// 2. Wire the platform composition.
services.AddPlatformIdentityLifecycle();

// 3. Map the lifecycle endpoints.
app.MapPlatformRefreshTokenRotation();
app.MapPlatformRefreshTokenRevocation();
app.MapPlatformPasswordRecoveryInitiation();
app.MapPlatformPasswordRecoveryCompletion();
app.MapPlatformTwoFactorChallenge();
app.MapPlatformTwoFactorVerification();
app.MapPlatformImpersonationStart();
app.MapPlatformImpersonationEnd();
```

The platform never replaces the consumer's authentication scheme, claim
projection, or identity schema.

## Refresh-token rotation

- `IRefreshTokenStore` — application-owned store. `ConsumeAsync` MUST be
  linearizable across concurrent callers presenting the same handle; at most
  one caller observes `Succeeded`, the others observe `Replayed` (and the
  family is revoked).
- `IRefreshTokenService` / `DefaultRefreshTokenService` — composes the
  application store with the platform `IIdentityAuditHook`.
- `RefreshToken` / `RefreshTokenRotation` — opaque handles. The platform
  recommends hashed-at-rest application storage and never persists handles
  on the consumer's behalf.
- `InMemoryRefreshTokenStore` — deterministic, linearizable, non-production
  store used by tests to prove the concurrency contract.

## Password recovery

- `IPasswordRecoveryService.InitiateAsync(identifier)` — MUST return the
  same `Succeeded` outcome for unknown and known subjects to prevent user
  enumeration. The returned `PasswordRecoveryChallenge` is an opaque handle
  with a server-side expiry.
- `IPasswordRecoveryService.CompleteAsync(challengeId, code, newPassword)` —
  `Replayed` after a successful completion, `Expired` past the issued window,
  `PolicyDenied` for mismatched codes.

## Two-factor

- `ITwoFactorService.IssueAsync(subjectId)` — returns an opaque
  `TwoFactorChallenge` with the consumer-supplied channel list and an
  application-controlled expiry. The platform never knows the subject's
  enrolled factors.
- `ITwoFactorService.VerifyAsync(challengeId, code)` — `Succeeded` on a
  match, `PolicyDenied` for mismatches, `Expired` past the window,
  `InvalidHandle` for unknown challenges.

## Impersonation

- `IImpersonationPolicy` — application-supplied authorization decision.
  Without a registered policy, every `StartAsync` call is denied and the
  service records an `identity.impersonation.denied` audit event.
- `IImpersonationService.StartAsync(request)` — returns an `ImpersonationGrant`
  on `Succeeded`. `EndAsync(grantId)` removes the grant; `GetActiveAsync`
  returns the `ImpersonationContext` for the current caller or `None` when
  no grant is active.
- `ImpersonationAuthorizationRequest` — `CallerSubjectId`,
  `TargetSubjectId`, required `Reason`, and `Duration`.

## Outcome codes

`IdentityLifecycleOutcome` enumerates `Succeeded`, `InvalidHandle`, `Expired`,
`Revoked`, `Replayed`, `PolicyDenied`, `PreconditionNotMet`,
`ProviderUnavailable`, `InvalidRequest`, and `Unknown`. `IdentityLifecycleResult<T>`
wraps every contract result. The platform requires stable codes — no
credential values, hashes, user-enumeration signals, or provider response
details ever appear in the outcome.

## Security requirements

- Refresh-token handles must be hashed at rest in the application store.
  The platform's `InMemoryRefreshTokenStore` is intentionally non-production.
- Password-recovery challenges must be one-time and expire server-side.
- Two-factor codes must be delivered through an application-controlled
  channel; the platform only carries the opaque handle.
- Impersonation is fail-closed. The application must register an
  `IImpersonationPolicy` before any grant can be created.
- Every lifecycle method records an `IdentityAuditEvent` (issued / rotated
  / revoked for refresh; denied / started / ended for impersonation).

## Migration from the starter kit

1. Add the four application-owned stores/policies/services. Keep the
   existing `ApplicationUser`, Identity DbContext, sign-in manager, and
   migrations unchanged.
2. Register them with the platform composition via
   `AddPlatformIdentityLifecycle`.
3. Map the platform endpoints where appropriate, or call the
   `IIdentityLifecycleCoordinator` from existing controller actions.
4. Rollback is removing `AddPlatformIdentityLifecycle`; existing routes
   continue to work because the platform never mutates the consumer's
   authentication scheme, claim projection, or identity tables.
