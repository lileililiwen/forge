# Proposal: jenkins-publish-integration

## Why

Forge needs a controlled deployment path for the Mac product environment without
making that Mac a source-code or automation owner. GitHub remains the canonical
code repository, Forge performs builds on Linux, and the Mac supplies only the
runtime and persistent runtime data.

## What Changes

- Add `mac-runtime` as a Forge deployment target backed by the generic
  `forge-deploy-executor/0.1.0` contract.
- Run the Mac adapter on Linux and send only argument-array Docker operations
  over SSH.
- Keep release manifests immutable and reference container images by digest.
- Run the existing Forge container build stage on Linux through the local
  container-builder contract; GitHub Actions is not part of the build path.
- Treat the existing Mac-local scripts and Jenkins jobs as deprecated recovery
  material, not as the deployment implementation.

## Package Boundary and Split Assessment

This outcome is split into two packages:

| Package | Owner | Responsibility |
|---|---|---|
| `jenkins-local/openspec/changes/mac-runtime-only-deployment` | `jenkins-local` | Linux-side Mac runtime adapter, runtime-only installer, and runtime boundary tests |
| `jenkins-publish-integration` | `forge` | Forge target registration, project-root handoff, local Linux build integration, and generic executor wiring |

The packages are sequentially dependent: the adapter contract and runtime
manifest are defined first, then Forge consumes that stable contract. They are
not merged into one repository because the adapter is an operational boundary,
while Forge is the generic deployment controller.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence | Decision |
|---|---|---|
| Existing Forge deploy executor | `src/deploy/{mod,engine}.rs`, `docs/adapter-contracts/deploy-executor.md` | Extend with `mac-runtime`; preserve the generic JSON envelope |
| Existing Forge container release stage | `src/release` container adapter | Reuse for Linux-side builds; do not create a GitHub build workflow |
| `jenkins-local` Mac scripts/jobs | `/Users/allen/jenkins/scripts`, `projects`, Jenkins job list | Keep only for migration/recovery; do not invoke or copy them in the new path |
| Workspace Governance | policy and eligibility provider | Consume as policy input; it does not own deployment execution |

## BFS Impact Map

| Affected surface | Impact |
|---|---|
| Forge manifest | Accept `kind: mac-runtime` |
| Forge executor | Pass the project root as the adapter working directory |
| Forge release | Build locally on Linux and produce a release manifest |
| Linux adapter | Pull, replace, run, and observe a named Mac container over SSH |
| Mac filesystem | Retain only images, volumes, logs, runtime configuration, and secrets |
| GitHub | Remains code history and review source; no build minutes are required |
| Jenkins | Migration/recovery reference only; no new deployment dependency |

## Capabilities

1. Build a release on Linux using Forge's local container stage.
2. Deploy an immutable image to the Mac runtime without copying source or
   scripts.
3. Observe the named runtime container through a read-only SSH/Docker command.
4. Preview all remote commands with a side-effect-free dry run.

## Non-goals

- Building in GitHub Actions.
- Maintaining deployment scripts, source checkouts, or Jenkins job logic on the Mac.
- Replacing Workspace Governance with deployment code.
- Making the Mac the only supported runtime provider.
- Moving runtime data or private secrets into GitHub.
