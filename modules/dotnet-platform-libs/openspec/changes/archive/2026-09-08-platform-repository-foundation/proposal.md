## Why

The shared platform needs an isolated repository with predictable build, package, test, and contribution conventions. Without this foundation, shared code will be copied into applications or coupled to one application's repository.

## What Changes

- Define the repository structure and supported target frameworks.
- Add central MSBuild and package-management defaults.
- Add build, test, pack, and validation documentation.
- Establish independent package boundaries without implementing business behavior.

## Capabilities

### New Capabilities

- `platform-repository-foundation`: A buildable, testable, versionable repository for opt-in .NET platform packages.

### Modified Capabilities

## Impact

Adds repository-level configuration, initial project files, quality gates, package metadata, and CI-ready commands. No consuming application is changed.

## Context

The workspace contains many projects across .NET 8 and .NET 10. The repository must support gradual adoption and must not force all applications to upgrade together.

## Goals / Non-Goals

**Goals:**

- Make the platform repository independently buildable and packable.
- Keep package boundaries explicit.
- Make the first implementation change safe for later AI-assisted work.

**Non-Goals:**

- Implement billing, Stripe, persistence, or application migrations.
- Reproduce the FullStackHero starter kit.
- Create a runtime service.

## Decisions

- Use one repository with multiple focused projects.
- Start with .NET 8-compatible production libraries; add .NET 10 targeting only when justified by a package requirement.
- Use central package version management.
- Treat package adoption as opt-in and independently versioned.

## Risks / Trade-offs

- Supporting multiple target frameworks can constrain APIs; pure contracts must stay dependency-light.
- A monorepo requires package ownership discipline; architecture tests and documentation will enforce boundaries.
