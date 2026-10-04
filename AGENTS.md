# Agent Instructions

## Scope

This repository contains reusable .NET libraries, not application features. Keep APIs small, explicit, and independently adoptable. Do not import a whole starter-kit architecture into this repository.

## Boundaries

- `Platform.Core` must remain free of ASP.NET Core and EF Core dependencies.
- `Platform.AspNetCore` may depend on `Platform.Core`, but not on product code.
- `Platform.Billing.Contracts` contains normalized subscription and entitlement contracts only.
- `Platform.Testing` may depend on the public platform contracts and test packages, but production packages must not depend on it.
- Product-specific persistence, migrations, provider IDs, invoice rules, and plan names stay in applications.

## Workflow

Implement one active OpenSpec change at a time using this exact sequence:

1. Select one active change with `openspec list`.
2. Implement only that change and its tests.
3. Update its `tasks.md` as tasks complete.
4. Verify with relevant tests and strict OpenSpec validation.
5. Archive the completed change.
6. Commit 1: implementation, tests, archive, and related generated specs only.
7. Update `HANDOFF.md` with completion evidence and the next change.
8. Commit 2: only the `HANDOFF.md` update.
9. Stop; do not start another change or push.

Incomplete or blocked work must not be claimed complete. Record the exact failed command and the next action in `HANDOFF.md`.

## Quality gates

- nullable reference types enabled;
- warnings treated as errors;
- public APIs have tests and XML documentation where useful;
- package dependencies are centrally versioned;
- no public API change without an OpenSpec change;
- `git diff --check` and strict OpenSpec validation must pass before completion.

## Current state

All ten OpenSpec changes listed in `ROADMAP.md` Phases 1, 2, 3, 4, and 5 are implemented and archived, along with the Phase 4 additions (`platform-admin-capability`, `platform-billing-provider-abstractions`, `platform-billing-provider-adapters`, `platform-ai-provider-abstractions`, `platform-notifications-sms`, `platform-ui-design-system`, `platform-application-starter`), the Phase 5 web-edge integrations (`platform-web-edge`, `platform-web-runtime-foundation`, `platform-persistence-efcore`, `platform-identity-authorization`), and the gap-audit changes (`platform-web-api-versioning`, `platform-identity-lifecycle-contracts`, `platform-tenant-lifecycle-contracts`, `platform-consumer-adoption-conformance`, `platform-testing-toolkit`). `Platform.Testing` and the per-component `*.Testing` packages exist; production projects do not reference them (enforced by `Platform.Architecture.Tests`). The `Platform.Testing.AspNetCore` package ships `PlatformTestWebApplicationFactory<TEntryPoint>`, which boots the consumer's real application over `TestServer` from a named entry point (one host per instance; no application of its own, and no silent fallback), and is itself a test-only package. The framework-neutral Phase 3 packages — `Platform.Jobs`, `Platform.Mailing`, `Platform.Eventing`, `Platform.Idempotency`, and `Platform.RateLimiting` — do not reference ASP.NET Core, EF Core, scheduling engines, mail providers, templating engines, RabbitMQ, StackExchange.Redis, or VisualFlow projects (each guarded by a dedicated architecture test). `openspec list` is empty; the next work, if any, starts with a fresh OpenSpec proposal. The current handoff is in `HANDOFF.md` and the per-package reference is in `docs/packages.md`.

## Git workflow

- Work on `main`. Do not create a branch or a worktree unless the user explicitly asks.
- Never push. The user owns publishing.
- Stage explicit paths; never `git add -A`.
