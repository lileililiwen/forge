# Proposal: portable Forge binary distribution

## Why

Forge already builds a host-specific archive and checksum locally, and its CI
uploads that pair as a short-lived workflow artifact. It does not publish a
versioned release asset. Generated project CI therefore compiles Forge from a
pinned Git source revision, which requires Cargo, the source repository and a
long compile on every project runner. The completed Gate template package
named this distribution package as its direct follow-up.

## What Changes

- Publish Forge's versioned Linux x86_64 archive and SHA-256 file as a GitHub
  release only after the full Forge CI job graph succeeds for the matching
  `v< Cargo.toml version >` tag.
- Preserve the existing local package, install and smoke commands; use their
  output as the release input instead of inventing a second archive format.
- Make the existing contract-parity CI job obtain the source revision recorded
  in `contracts/manifest.json`, so the full CI graph is runnable without an
  adjacent `platform-contracts` checkout.
- Add immutable standard-pack versions for service and client profiles whose
  GitHub Actions workflows install a pinned Forge release, verify the archive
  digest and then invoke the existing standard verifier and `forge gate .`.
- Advance new-project defaults to those versions and document the initial
  release prerequisite. Existing snapshots keep their current bytes until an
  explicit standard upgrade.
- Correct the consolidation roadmap to mark the Gate reconciliation complete
  and this package active/completed as it progresses.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-portable-binary-distribution` | A CI-verified Forge version is available as a checksummed release asset and generated CI can install that exact version without compiling Forge | Forge; Rust, POSIX shell, GitHub Actions YAML | Existing `scripts/package.sh` archive name and `.sha256` format; GitHub release tag `v<semver>`; standard-pack snapshots | `artifact-and-ci-baseline`, `forge-ci-precommit-templates`, completed Gate reconciliation | Tag mismatch blocks; all CI jobs pass before publication; release assets match the package output; generated workflow rejects a corrupt digest before running Gate |
| `forge-platform-target-matrix` | Publish and verify additional OS/architecture artifacts for local developer hosts | Forge; separate packaging/runtime matrix | New platform archive formats and native smoke evidence | This package | Native build/install/smoke for each declared target |
| `forge-platform-schema-ownership` | Remove long-term mirrored schema maintenance and sibling source comparison | Forge plus `platform-contracts` ownership boundary | Versioned schemas and validators with Forge ownership | Current release package is independent; remains next consolidation item | No sibling checkout needed for Forge schema operations |

Only the first row is in scope. Release publication and generated CI adoption
share one owner, artifact identity, version pin and acceptance path; separating
them would publish an artifact that current generated projects still cannot
consume. Cross-platform distribution has its own target/build/smoke lifecycle
and remains a follow-up. Schema ownership is independent. No sibling source or
runtime is imported into Forge by this package.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge's current artifact baseline | `scripts/package.sh`, `scripts/checksum.sh`, `scripts/install.sh`, `.github/workflows/ci.yml` `artifact` job | Host archive format, SHA-256 sidecar, scratch install and binary smoke are already implemented and tested | Current CI artifact is not a durable GitHub release; installer has no network behavior | Forge owns package bytes and project release workflow | **extend shared owner** |
| Forge's versioned CI templates | `standards/baseline-service/1.3.0/ci/verify.yml`, `standards/baseline-client/1.0.0/ci/verify.yml`, `src/standard/snapshot.rs::all_packs` | Pinned, receipt-protected templates and conflict-safe upgrades | Templates compile Forge from a Git checkout; they do not install a release asset | Forge owns and versions generated template bytes | **extend shared owner** |
| Workspace Governance | `workspace-governance/.github/workflows/governance.yml`, `workspace-governance/templates/`, `workspace-governance/HANDOFF.md` | Workspace policy and reusable project bootstrap/verifier guidance | It does not own Forge executable builds or release assets; its checkout is ahead of origin and is read-only here | Workspace Governance owns governance policy; Forge owns its executable | **keep local** |
| Platform Contracts | `platform-contracts/manifest.json`, Forge `contracts/manifest.json`, `scripts/contract-parity.sh` | The manifest records the exact upstream revision consumed by Forge's byte-parity check | It defines wire schemas, not executable packaging; current parity CI assumes a sibling checkout | Platform Contracts owns schemas; Forge's workflow reads the pinned public source for parity | **adapt through a generic adapter** |
| .NET and Rust platform libraries | `dotnet-platform-libs/.github/workflows/release.yml`, `rust-platform-libs/HANDOFF.md` | Separate package identities, independently owned release workflows, explicit/manual release operations | NuGet/Cargo publication has different artifacts, secrets, registries and lifecycle from Forge's CLI archive; both checkouts contain unrelated dirty files | Each library repository owns its own packages and publication | **keep local** |

The sibling repositories were inspected read-only. No sibling file will be
changed; no credentials, release tags or remote publication will be used.
Forge's generated projects consume only Forge's published archive and retain
their native build/test tooling without a source checkout.

## User Experience and Interface Impact

The release maintainer starts a release by pushing the matching `v<semver>`
tag. The GitHub Actions run shows version validation, the normal CI job results,
artifact packaging and the release upload as separate named steps. A mismatch,
failed required job, missing asset or existing release ends in a visible
failure; the release job never overwrites an existing version. Recovery is to
correct the source and publish a new version/tag, not replace an existing
asset.

A project maintainer sees a named “Install pinned Forge release” step in the
generated Actions workflow. On success, the workflow adds the extracted
binary directory to `GITHUB_PATH` before running the existing verifier and
Gate. Download, checksum, archive-layout and binary failures appear as bounded
step errors before Gate runs. The workflow remains `contents: read`; only the
Forge release job receives `contents: write`. There is no new Forge portal
screen or local interactive flow. GitHub Actions supplies the external UI;
stable job/step names and actionable diagnostics are the applicable interface
and accessibility surface. No responsive or keyboard-specific Forge UI
changes apply.

## BFS Impact Map

| Surface | Impact and invariant |
|---|---|
| Requirements and specs | Add the version-tag publication, checksum and generated-consumer requirements; preserve the existing local packaging contract. |
| Release workflow | `.github/workflows/ci.yml` publishes only on a matching version tag and only after every existing verification job, including contract parity, passes. The read-only contract source comes from the revision in Forge's mirror manifest. |
| Artifact contract | Keep `forge-<version>-x86_64-unknown-linux-gnu.tar.gz` and its sibling `.sha256`; release tag is `v<version>`. Reject tag/version mismatch and duplicate release assets. |
| Generated projects | Add service/client pack versions; their installer checks the checksum filename and digest before extraction, installs to a runner-local directory, and then existing `forge gate .` remains the sole Gate entry point. |
| Callers and compatibility | `forge new`, standard install/check/diff/upgrade continue to use compiled-in descriptors and ownership receipts. Existing pack bytes/defaulted projects are unchanged; new generation selects the new version. |
| Configuration and persistence | No registry, project manifest, SQLite or cross-project contract schema changes. Release version is pinned in immutable pack assets and archive/tag names. |
| Failure boundaries | CI failures block publication. Invalid tag, absent release asset, bad checksum, unexpected archive layout, missing tools, network error and existing release fail closed before publication or Gate execution. |
| Tests | Add release workflow and installer contracts, asset/package checks, default-pack and receipt/upgrade coverage, and a local offline installer fixture. |
| Dependencies and security | No new Rust crates or runtime service. Use GitHub's built-in `gh` CLI for publication; confine `contents: write` to the release job; generated workflows use `curl`, `sha256sum` and `tar` on the declared Ubuntu runner. No credentials enter artifacts or logs. |
| UI, native build and provider evidence | No Forge browser UI, new profile, native scaffold or external publisher adapter is changed. GitHub tag/release interaction is explicitly external and remains operator-triggered. |

## Capabilities

- `forge-portable-binary-distribution`
- `standard-pack-registry-and-snapshots` (consumer version delta only)
- `artifact-and-ci-baseline` (tag-gated release delta only)

## Non-goals

- Building Windows, macOS, Linux ARM or musl binaries; the current generated
  Forge Gate workflows run on Ubuntu x86_64. Additional targets require native
  build/install/smoke evidence in a follow-up package.
- Publishing a release during this change, creating/pushing a tag, changing
  repository visibility, or modifying any sibling checkout.
- Replacing Forge's release engine for product repositories or introducing a
  generic release service, package registry, self-updater or latest-version
  resolver.
- Changing DriftWatch compatibility, the Gate baseline, service verifier,
  local pre-commit behavior, or project native build/test ownership.
- Changing existing standard pack versions or silently overwriting user-edited
  files during an upgrade.
