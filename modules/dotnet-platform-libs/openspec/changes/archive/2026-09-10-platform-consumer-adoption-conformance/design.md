## Context

The platform documentation already states that consumers should use packed packages and that production projects must not reference testing packages. The adoption path must be safe for heterogeneous repositories under `/home/paul/code`, without assuming a shared solution or a common database.

## Goals / Non-Goals

**Goals:**

- Prove package behavior through the same artifacts consumers install.
- Make one-package-at-a-time adoption and rollback explicit.
- Detect forbidden dependency direction and accidental provider coupling in consumer fixtures.

**Non-Goals:**

- Rewrite all 26+ projects.
- Add cross-repository `ProjectReference` links.
- Mandate one application architecture, database, provider, or deployment model.

## Decisions

- Use a local/private NuGet feed populated by `dotnet pack`; pin exact package versions in the fixture.
- Keep the fixture solution-excluded if it has different restore requirements, but run it in CI through a dedicated script.
- Represent capabilities and package dependencies in a checked-in manifest generated from project metadata and reviewed on change.
- Require a rollback test that restores the prior package version and rebuilds the consumer.
- Start with a non-critical pilot consumer and record the selected package boundary before wider adoption.

## Risks / Trade-offs

- [Risk] A local feed hides real feed behavior → add an optional private-feed job and verify package metadata/signatures.
- [Risk] Consumer fixture overfits one application → keep it minimal and add contract scenarios rather than application business logic.
- [Risk] Upgrade failures are discovered late → require previous/current matrix testing for public contract packages.

## Migration Plan

1. Formalize the fixture and manifest.
2. Consume one stable package from a packed local feed.
3. Run upgrade and rollback checks.
4. Select one real non-critical application and repeat the same process manually.
