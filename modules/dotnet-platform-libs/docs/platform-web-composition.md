# Platform.Web.Composition

Explicit, provider-neutral ASP.NET Core module composition. The platform
owns the module contract, deterministic ordering, duplicate rejection,
and the per-host registry snapshot. The application owns module
implementations, middleware ordering relative to non-module middleware,
and all handler/validation/persistence conventions.

## Packages

| Package | Purpose |
| --- | --- |
| `Platform.Web.Composition` | Optional ASP.NET Core composition: `IPlatformWebModule`, `AddPlatformWebModule`, `PlatformWebModuleRegistry`, `UsePlatformWebModules`, `MapPlatformWebModules`. Depends on `Platform.Core` plus the `Microsoft.AspNetCore.App` framework reference. No Mediator, FluentValidation, EF Core, or product references. |

## Contracts

`IPlatformWebModule` declares a stable `Name`, a deterministic `Order`,
and three hooks: `ConfigureServices` (required), `ConfigureMiddleware`
(default no-op), and `MapEndpoints` (default no-op). The interface — not a
base class — is the integration seam; modules are plain classes with a
public parameterless constructor.

## Registration

Modules are registered explicitly: `services.AddPlatformWebModule<T>()`
(or the `Type` overload for an explicitly enumerated set). Each call
instantiates the module, validates its name, invokes
`ConfigureServices`, and records the instance as an
`IPlatformWebModule` singleton. Duplicate types or duplicate names fail
deterministically with `InvalidOperationException` naming the conflict.
Nothing is scanned: a module type that is never passed to registration
is never instantiated or mapped.

`PlatformWebModuleRegistry.FromProvider(provider)` builds the immutable
ordered snapshot (`Order`, then `Name` ordinal) from the host service
set. State lives in DI, so two hosts built in the same process keep
independent module sets.

## Pipeline

`UsePlatformWebModules(app)` and `MapPlatformWebModules(endpoints)` are
opt-in. Without them, registered modules contribute no middleware or
endpoints. Each invocation runs every registered hook at most once, in
registry order. Module order is the only platform guarantee: keep core
platform middleware (correlation, errors) outside modules, and order
non-module middleware explicitly around the module call.

## Deliberate differences from the starter loader

The starter kit `ModuleLoader` (`FSH.Framework.Web.Modules`) differs in
every composition decision:

- Discovery: the starter scans `AppDomain` assemblies for
  `FshModuleAttribute`; the platform requires explicit registration
  calls and rejects unregistered types by construction.
- Coupling: the starter registers FluentValidation validators for every
  scanned assembly; the platform registers no validators, Mediator
  handlers, persistence, or product services.
- State: the starter keeps modules in a static process-global list with
  a first-wins loaded flag, so parallel hosts and tests share (and can
  poison) one registry; the platform keeps the snapshot in DI, scoped
  to each host.
- Ordering: the starter orders by attribute `Order` then type name; the
  platform orders by the module-declared `Order` then stable `Name`.

## Migration from the starter loader

1. Implement `IPlatformWebModule` on each module (move the
   `ConfigureServices` body across; keep `MapEndpoints`; move
   `ConfigureMiddleware` only where the module had one).
2. Replace the `AddModules(assemblies)` call with one
   `AddPlatformWebModule<T>()` per module, in the order the host
   should run them.
3. Replace `UseModuleMiddlewares()` with `UsePlatformWebModules()` and
   `MapModules()` with `MapPlatformWebModules()`, keeping surrounding
   platform middleware where it was.
4. Register validators/handlers explicitly where they were previously
   scanned in; nothing is picked up implicitly anymore.
5. Rollback: remove the package reference and restore the previous
   registration calls. No schema or data migration is involved.

## Security

- Duplicate or empty module names fail closed at startup with a message
  naming the conflicting type — no request is served with an ambiguous
  module set.
- Registry contents are host-scoped; one host cannot observe or invoke
  another host's modules through the platform surface.
- Module names and order values are public routing metadata; do not
  embed secrets or connection strings in modules.
