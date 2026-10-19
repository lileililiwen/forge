## Why

The starter kit makes adding a module convenient, but its loader couples consumers to Mediator, FluentValidation, static process-global state, and a fixed module architecture. Small projects need an explicit composition seam that can register only what they own.

## What Changes

- Add a small web composition contract for service registration, middleware, and endpoint mapping.
- Support explicit module registration with deterministic order and duplicate detection.
- Keep Mediator, FluentValidation, module discovery policy, and application feature conventions outside the package.

## Capabilities

### New Capabilities

- `web-composition`: explicit, provider-neutral application module composition.

### Modified Capabilities

- None.

## Impact

New optional ASP.NET Core package and public module interfaces. Existing `Platform.Web` and `Platform.Starter` behavior remains unchanged unless a consumer opts into the new package.

## Non-Goals

- No automatic AppDomain scanning.
- No static mutable registry.
- No CQRS, validation, authorization, or persistence conventions.
