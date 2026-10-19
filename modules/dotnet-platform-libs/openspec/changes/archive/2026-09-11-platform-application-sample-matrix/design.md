## Context

The platform has a composite sample and a template scaffold, while the starter kit has one intentionally integrated full stack. Consumers need evidence that platform packages can be adopted one at a time without inheriting that stack.

## Goals / Non-Goals

**Goals:** provide five focused samples, build each independently, record package/version ownership, and test the smallest meaningful request or service flow.

**Non-Goals:** maximize feature breadth, share application domain entities, or create a second product.

## Decisions

- Keep samples under `samples/` with separate projects and minimal source files.
- Use one stable endpoint or service assertion per sample; avoid duplicating module implementations.
- The EF Core sample owns its context and migration fixture; the identity and tenancy samples own their stores/configuration.
- The provider sample uses a deterministic fake or local adapter, never live credentials.
- Add a matrix manifest documenting target framework, package references, external requirements, and rollback operation.
- Run samples in CI where infrastructure is available; classify unavailable external services as blocked rather than green.

Alternatives considered: expanding the existing sample into a full app would hide adoption boundaries; using the starter repository as a submodule would create a coupled test; documentation-only examples would not prove compilation or registration.

## Risks / Trade-offs

- [Risk] Sample duplication increases maintenance → keep flows tiny and reuse only public packages.
- [Risk] Samples accidentally become recommended architecture → state ownership and non-goals beside every sample.
- [Risk] External provider checks are flaky → use deterministic fakes and separate optional live checks.

## Migration Plan

No consumer migration. Samples are reference/conformance projects and can be removed independently without affecting package consumers.

## Open Questions

None; the matrix deliberately begins with five adoption stages.
