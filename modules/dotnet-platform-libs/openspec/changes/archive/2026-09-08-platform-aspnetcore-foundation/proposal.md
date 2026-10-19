## Why

Web applications repeat safe HTTP error mapping, request correlation, request context access, and health-check registration. These are suitable for an opt-in ASP.NET Core package after the framework-neutral core contracts exist.

## What Changes

- Add ASP.NET Core dependency-injection extensions.
- Map core failures to ProblemDetails without leaking sensitive details.
- Provide request correlation access.
- Provide explicit health-check registration helpers.

## Capabilities

### New Capabilities

- `platform-aspnetcore-foundation`: Optional ASP.NET Core integration for the shared core contracts.

### Modified Capabilities

## Impact

Adds the first framework-specific package and its tests. It depends on `Platform.Core` but does not depend on EF Core, Stripe, or a product application.

## Context

Applications currently use different web stacks and error conventions. This package must be additive and opt-in rather than requiring a host rewrite.

## Goals / Non-Goals

**Goals:**

- Make common web behavior consistent for newly adopted projects.
- Preserve application control over endpoint routing and authentication.

**Non-Goals:**

- Replace authentication or authorization.
- Provide a complete API framework.
- Require a specific logging provider.

## Decisions

- Use standard ASP.NET Core ProblemDetails primitives.
- Use explicit extension methods with no hidden global registration.
- Generate or accept correlation IDs through a documented request-context contract.

## Risks / Trade-offs

- Middleware order matters; registration helpers must document ordering.
- Existing applications may need adapters for their current error types.
