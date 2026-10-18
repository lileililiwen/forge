# Proposal: Govern reusable platform capabilities and release evidence

## Why

The portfolio now has shared runtime libraries, a Gate executor, and repeated
release scripts, but no common capability vocabulary showing which projects
need identity, admin, jobs, billing, storage, or release evidence. Without that
vocabulary, adoption becomes package copying rather than controlled reuse.

## What Changes

- Add a versioned capability declaration for projects.
- Add profile rules that map capabilities to required checks and evidence.
- Standardize release evidence fields: source revision, version, toolchain,
  artifacts, checksums, SBOM, provenance, and publication state.
- Keep declarations and exceptions in products while shared implementations
  remain in platform repositories.

## BFS Impact Map

- **Capabilities:** project capability profiles and release evidence.
- **Consumers:** Forge, Driftwatchdog, .NET/Rust platforms, Jenkins-local, and
  business projects.
- **Data:** metadata and evidence vocabulary only; no product runtime data.
- **Failure:** missing required capability evidence is `REVIEW_REQUIRED`, not
  an inferred pass.
- **Compatibility:** existing `.project.json` and `.ai-gate` declarations stay
  valid; capability fields are additive.

## Capabilities

- `platform-capability-manifest`.
- `release-evidence-vocabulary`.

## Non-goals

- No automatic dependency injection or package installation.
- No deployment promotion from metadata alone.
- No replacement for Driftwatchdog, Jenkins, Forge, or native toolchains.
