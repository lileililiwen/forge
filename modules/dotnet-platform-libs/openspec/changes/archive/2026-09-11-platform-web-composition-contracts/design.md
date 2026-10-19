## Context

`Platform.Starter` composes existing capabilities, but applications still repeat composition code when they have a few local modules. The starter kit's `ModuleLoader` scans assemblies, registers validators, instantiates modules, and stores them in static state. That behavior is too implicit for reusable libraries.

## Goals / Non-Goals

**Goals:** offer explicit module registration on the ASP.NET Core builder, deterministic ordering, per-host state, and separate service/middleware/endpoint hooks.

**Non-Goals:** discover application assemblies, own module lifetimes, register handlers or validators, or decide middleware ordering outside the registered module order.

## Decisions

- Add `Platform.Web.Composition` depending on `Platform.Core` and ASP.NET Core abstractions.
- Define `IPlatformWebModule` with a stable name/order and `ConfigureServices`, `ConfigureMiddleware`, and `MapEndpoints` methods.
- Store registrations in DI as an immutable ordered collection built during registration; no static fields.
- Require explicit `AddPlatformWebModule<T>()` calls and reject duplicate names or types.
- Expose `UsePlatformWebModules` and `MapPlatformWebModules` as opt-in extensions. Endpoint mapping is idempotent per application pipeline invocation.

Alternatives considered: copying the starter loader would make behavior hidden and unsafe in tests; a source generator would add tooling complexity; an event-based registration API would make ordering and failure behavior harder to reason about.

## Risks / Trade-offs

- [Risk] Consumers register middleware in the wrong relative order → document that module order is the only platform guarantee and keep core platform middleware separate.
- [Risk] Modules perform expensive work during registration → require registration methods to configure services/routes only and test duplicate/ordering behavior.
- [Risk] Consumers expect automatic discovery → explicitly require assembly references and registration calls.

## Migration Plan

Consumers may wrap existing module setup in `IPlatformWebModule` one module at a time. Rollback removes the package and returns to direct registration; no runtime data changes occur.

## Open Questions

None; automatic discovery is deliberately deferred.
