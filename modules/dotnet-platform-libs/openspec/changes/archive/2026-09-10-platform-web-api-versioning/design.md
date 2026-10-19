## Context

The starter implementation in `src/BuildingBlocks/Web/Versioning/Extensions.cs` registers API versioning with a default `1.0`, URL-segment reading, reporting, and API Explorer group names. The platform already has separate `Platform.Web.OpenApi` and web-edge packages.

## Goals / Non-Goals

**Goals:**

- Provide a small opt-in registration package with safe defaults.
- Allow applications to select URL, header, query, or composite readers.
- Keep version metadata available to OpenAPI generation.

**Non-Goals:**

- Discover or assign versions to application endpoints automatically.
- Own endpoint routes, controller conventions, or deprecation schedules.
- Force versioning on existing consumers.

## Decisions

- Use `Asp.Versioning.Http` and `Asp.Versioning.Mvc.ApiExplorer` directly in the adapter package.
- Default to URL-segment reading and version `1.0` only when the consumer opts in; expose configuration for other readers.
- Register through `TryAdd`/options patterns where possible so application configuration wins.
- Keep OpenAPI document naming in the existing OpenAPI package, with documented integration rather than a package cycle.

## Risks / Trade-offs

- [Risk] URL versioning changes route contracts → opt-in only and require explicit migration documentation.
- [Risk] Multiple readers create ambiguous requests → validate reader configuration and test precedence.
- [Risk] Asp.Versioning major-version drift → central package pinning and release compatibility checks.

## Migration Plan

1. Add the package without changing existing web defaults.
2. Pilot it on one non-critical application with one versioned route group.
3. Validate generated OpenAPI documents and client routes.
4. Adopt per application after rollback to unversioned routes is verified.
