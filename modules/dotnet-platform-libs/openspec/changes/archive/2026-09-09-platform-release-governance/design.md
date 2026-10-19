## Context

The platform contains 65 source projects and many independently adoptable packages. `Directory.Build.props` enables nullable reference types, deterministic builds, and warnings-as-errors, but the repository has no `.github/workflows` release gate. The starter kit provides a useful reference with path-scoped CI and template smoke tests; its source-ownership distribution model is not appropriate here.

## Goals / Non-Goals

**Goals:**

- Prove the exact NuGet artifacts consumed by applications.
- Detect public API breaks, vulnerable dependencies, and unreviewed release metadata.
- Make .NET 8 the current baseline while documenting the .NET 10 compatibility path.

**Non-Goals:**

- Automatically migrate or update all applications.
- Require a public NuGet feed, a specific CI vendor, or a specific signing service.
- Make every package depend on every adapter.

## Decisions

- Use a path-scoped CI workflow with serial `-m:1` restore/build/test/pack commands. This matches repository resource constraints and avoids parallel MSBuild failures.
- Validate packed artifacts through `tests/Platform.ConsumerConformance` or an equivalent fixture restored from a local package directory. A project reference is insufficient evidence.
- Add API compatibility checks only for packages with public contracts, using a checked-in baseline generated from the last release. Internal implementation-only projects can be excluded explicitly.
- Run `dotnet list package --vulnerable` or an equivalent audited restore with a documented exception process. Do not hide advisories with broad suppression.
- Keep signing and SBOM as release gates that can use CI secrets, while local builds remain usable without credentials.

## Risks / Trade-offs

- [Risk] Baselines become stale → require a deliberate baseline update in the release PR and review the generated API diff.
- [Risk] Vulnerability feeds are unavailable → distinguish unavailable checks from passing checks and fail release publication when the required audit cannot run.
- [Risk] Package metadata changes become noisy → centralize defaults and allow package-specific overrides only when documented.

## Migration Plan

1. Add local scripts and fixtures and run them without CI credentials.
2. Enable build/test/pack and strict spec validation on pull requests.
3. Add compatibility and vulnerability gates in warning mode with explicit findings.
4. Enable release blocking after the baseline and exception process are reviewed.
