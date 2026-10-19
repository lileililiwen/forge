# Proposal: Post-MVP readiness — package matrix, consumer guide, examples, license

## Why

The platform is complete (52 archived changes, 75 `src/` packages, 1,386 passing
tests) but its front door does not let a consumer adopt it:

- `README.md` (156 lines) lists only a subset of the packages with a one-line
  purpose each, then jumps to repository layout and `dotnet` commands. It is not
  an authoritative per-package matrix and its relationship to `docs/packages.md`
  is unclear.
- There is no inline versioning/consumer guide: SemVer policy, `VersionPrefix`,
  source-vs-package mode, central package management, upgrade/rollback, and the
  .NET 10 baseline live across several documents, none summarized where a
  consumer starts.
- There are no usage examples. A consumer sees a package list but no minimal
  "reference this, call this" snippet per concern.
- There is no `LICENSE` file and no README license section; the only terms are
  the `PackageLicenseExpression=MIT` metadata in `Directory.Build.props`.
- Adoption guidance is scattered (`docs/platform-product-adoption.md`, the
  adoption tool, `samples/`) with no single entry point.

Post-MVP readiness is the documentation front door, not new packages or API.

## What Changes

- Rewrite the README package section as an authoritative matrix grouped by
  concern (Core/Domain, Web/Composition/AspNetCore, Billing, Jobs, Mailing,
  Eventing, Caching, Storage, Quota, Identity, Tenant lifecycle, Testing, and
  optional adapters), each row naming kind, dependency direction, and the
  `docs/packages.md` surface reference.
- Add a versioning and consumer guide inline: the single version source
  (`VersionPrefix`), SemVer policy, the `net10.0` baseline, source-vs-package
  mode (workspace bootstrap), central package management, and upgrade/rollback,
  with links to `docs/release-governance.md`, `docs/dotnet10-migration-contract.md`,
  and `docs/workspace-consumer-bootstrap.md`.
- Add minimal usage examples per concern, verified against `samples/` and the
  starter template.
- Add a `LICENSE` file (MIT, matching the existing `PackageLicenseExpression`)
  and a README license section.
- Add one adoption entry point pointing at `docs/platform-product-adoption.md`,
  `tools/Platform.Adoption.Tool`, and `samples/matrix.json`.

## Public API impact

**None.** Documentation, a license file, and usage examples only. No `Platform.*`
type, method, package identity, or version line changes; `eng/public-api-baseline.txt`
and `eng/package-manifest.json` stay byte-identical.

## Package Boundary and Split Assessment

**Single outcome:** a consumer can select the right package, understand its
version and dependency direction, copy a minimal example, and know the reuse
terms.

**Included:** the README package matrix, the versioning/consumer guide, the
usage examples, the license section, the `LICENSE` file, the adoption pointers,
and the docs cross-links.

**Excluded:** any public API change, new package, version bump, consumer edit,
and runtime/deployment work.

**Split signals considered:** the matrix, guide, examples, and license share one
owner (this repository), one lifecycle (the docs front door), and one oracle (a
consumer follows the README to a referenced example). Splitting the license
leaves the packages legally unspecified; splitting the examples leaves the guide
abstract.

**Dependencies:** consumes the shipped `dotnet10-platform-baseline`,
`platform-consumer-adoption`, `platform-contract-conformance`, and the existing
per-package docs; blocks nothing.

## Sibling and Shared Architecture Reconnaissance

**Question:** should readiness be a shared manager/common capability across the
workspace, or stay local to this repository?

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository | `docs/packages.md`, `docs/release-governance.md` | authoritative per-package surface + versioning policy | the README does not surface them | dotnet-platform-libs | **extend docs; keep local** |
| this repository | `Platform.Adoption`, `tools/Platform.Adoption.Tool`, `openspec/specs/platform-consumer-adoption/spec.md` | the existing shared adoption diagnostic + packed-artifact conformance oracle | already a consumer-facing capability; a "readiness manager" would duplicate it | dotnet-platform-libs | **consume, do not add a manager** |
| mewo, somodanote (workspace apps) | their `README.md`, run scripts | a common README/readiness shape could be templated | app-specific product narrative and demo | each app | **keep local per app** |
| workspace-governance | `projects.json`, `profiles/`, `MATURITY_PROGRESS_SUMMARY.md` | cross-repo maturity/readiness profiles and metadata | owns workspace-level readiness, not this repo's package story | workspace-governance | **adapt**: report to its profile, do not fork it |
| platform-contracts | `schemas/`, `docs/` | cross-project contract schemas | no shared readiness artifact here | platform-contracts | **not applicable** |

**Decision (recorded):** readiness stays **repository-local documentation plus
the existing `Platform.Adoption` capability**. This repository is explicitly *not
a shared runtime service* (`openspec/config.yaml`), so a shared readiness manager
would contradict its smallest-independently-adoptable boundary; `Platform.Adoption`
already owns consumer diagnostics and conformance; and workspace-level maturity is
owned by `workspace-governance`. If a cross-repo template is later wanted, it is a
`workspace-governance` change, not a platform package.

## BFS Impact Map

- **Capabilities:** new `readiness`.
- **Systems touched:** `README.md`, a new `LICENSE`, and docs cross-links; usage
  examples referencing existing `samples/`, `templates/`, and `tools/`.
- **Contracts / packages:** none; no API, manifest, or baseline change.
- **Failure / boundary:** an example that does not compile, a matrix row that
  names a non-existent package, or a license mismatch with
  `PackageLicenseExpression` must fail review loudly.
- **Compatibility:** consumer-visible docs only; existing consumers, feed, and
  versions unchanged.
- **Tests:** `scripts/quality-gate.sh`, `scripts/conformance.sh`,
  `check-public-api.sh`, `generate-package-manifest.sh --check`, and strict
  OpenSpec validation must stay green; examples are checked against `samples/`.
- **Unaffected:** `src/**`, `tests/**`, package versions, and the feed.

## Capabilities

### New Capabilities
- `readiness`: the platform is selectable, versionable, exemplar-backed, and
  explicitly licensed for consumers.

### Modified Capabilities
- none.

## Non-goals

- No public API change, new package, new dependency, version bump, or release.
- No edit to any consumer repository; adoption is documented, not performed.
- No shared readiness manager/common capability (decision recorded above).
- No screenshots: this repository ships no user-facing UI; its consumer surface
  is package APIs, a read-only CLI, and docs. Visual capture would be noise, so
  it is recorded `NOT_APPLICABLE` with that justification and MUST NOT fail
  readiness.
- No runtime, deployment, signing, or SBOM evidence.

## Evidence Boundary

The README, examples, and license are documentation artifacts. They cannot prove
that any consumer compiles, runs, or ships against the packages; that evidence
stays with the consumer's own build and with the packed-artifact conformance
fixture. This change does not alter the version or baseline.