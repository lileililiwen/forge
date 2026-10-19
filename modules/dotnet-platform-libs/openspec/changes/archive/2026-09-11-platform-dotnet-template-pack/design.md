## Context

The existing `templates/platform-application-starter` has source files but no `.template.config` or pack project. Consumers therefore copy files manually. The target audience needs a minimal, inspectable starting point for small projects, not an attached framework runtime.

## Goals / Non-Goals

**Goals:** produce a `dotnet new`-installable NuGet template, support project-name and capability substitutions, include a testable generated project, and document package pinning and detachment.

**Non-Goals:** provide a wizard, update command, frontend, infrastructure orchestration, or application-domain code.

## Decisions

- Add a pack-only `Platform.Application.Template` project with `PackageType=Template`, no compiled library output, and content sourced from the existing minimal scaffold.
- Add `.template.config/template.json` with `sourceName`, `identity`, short name, symbols for application name and optional capabilities, and safe excludes.
- Generate a net8.0 minimal API by default with `Platform.Starter`; optional identity/EF Core references are explicit template symbols and remain application-owned.
- Keep generated source detached: no project reference or runtime dependency on the template pack.
- Validate by packing, installing into a temporary directory, generating each supported variant, and building/testing the result.

Alternatives considered: copying the full starter-kit pack would recreate the complexity problem; keeping a copy-only scaffold leaves adoption friction; a custom CLI would be unnecessary until template usage proves insufficient.

## Risks / Trade-offs

- [Risk] Template content drifts from package APIs → add generated smoke projects to CI and update the template with public API changes.
- [Risk] Template symbols create invalid combinations → constrain symbols and test every supported combination.
- [Risk] Package metadata leaks repository tooling → use an explicit tracked-content allowlist.

## Migration Plan

Existing consumers are unaffected. New consumers install the template; rollback is uninstalling it or deleting generated source. No application runtime dependency is added by installing the pack.

## Open Questions

None; minimal API plus optional test project is the first supported shape.
