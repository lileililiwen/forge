## Context

The platform repository is a new internal monorepo consumed by unrelated applications. The first release needs a low-friction local project-reference workflow and a later private NuGet workflow.

## Architecture

Create four initial projects under `src/`: `Platform.Core`, `Platform.AspNetCore`, `Platform.Billing.Contracts`, and `Platform.Testing`. Create matching focused test projects under `tests/` where behavior exists. `Platform.Core` and `Platform.Billing.Contracts` must not reference ASP.NET Core or EF Core.

`Platform.AspNetCore` may reference `Platform.Core`. `Platform.Testing` may reference the public platform contracts and test-only packages. Production projects must not reference `Platform.Testing`.

## Packaging and Versioning

Each library is packable with the same repository version by default, but package metadata must allow independent releases later. Consumers may begin with local `ProjectReference` entries and move to a private feed without changing namespaces.

## Verification

The foundation is verified by restoring, building, testing, packing, checking package dependency direction, running `git diff --check`, and running strict OpenSpec validation.
