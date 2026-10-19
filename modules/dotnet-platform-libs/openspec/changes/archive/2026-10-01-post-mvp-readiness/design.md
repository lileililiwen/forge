# Design: Post-MVP readiness for the .NET platform libraries

## 1. Implementation boundary

Repository `/home/paul/code/dotnet-platform-libs` — C#/.NET (SDK `10.0.400`,
`net10.0`), 75 `src/` packages.

Files to add/change: `README.md`; a new `LICENSE`; possibly a back-reference in
`docs/packages.md`; `.gitignore` only if needed.

Must not change: any `src/**` or `tests/**` code, `Directory.Build.props` /
`Directory.Packages.props`, `eng/package-manifest.json`,
`eng/public-api-baseline.txt`, `global.json`, `CHANGELOG.md` version lines,
`samples/**` behaviour, `templates/**` behaviour, or `openspec/specs/**` other
than the promoted change spec on archive.

## 2. Runtime and commands

Runtime: `dotnet` SDK `10.0.400`, `net10.0`.

Exact commands (run from the repository root):

- `dotnet restore Platform.sln`
- `dotnet build Platform.sln -c Release`
- `dotnet test Platform.sln -c Release --nologo`
- `dotnet pack Platform.sln -c Release --no-build --nologo`
- `./scripts/quality-gate.sh`
- `./scripts/conformance.sh`
- `./scripts/check-public-api.sh`
- `./scripts/generate-package-manifest.sh --check`
- `./scripts/package-inventory.sh`
- `openspec validate --changes --strict --no-interactive`
- License check: `test -f LICENSE`

## 3. Ownership and shared code

**Decision:** readiness stays repository-local; no shared manager. Dependency
direction is unchanged and restated in the README matrix: `Platform.Core` and
`Platform.Billing.Contracts` reference nothing; `Platform.AspNetCore` may
reference only `Platform.Core`; production packages never reference
`Platform.Testing*`; adapters reference their contract package. Package
ownership and identities are unchanged; no package is added. The smallest
adoptable boundary is preserved because the change adds no code a consumer is
forced to take.

## 4. Behavioral model

| Input/state | Result |
|---|---|
| README consulted for a package | The matrix row names kind, dependency direction, and a `docs/packages.md` anchor |
| README consulted for versioning | The single version source, SemVer policy, baseline, and source/package mode are summarized with links |
| README consulted for an example | A minimal snippet per concern, traceable to `samples/` or the template |
| License inspected | `LICENSE` exists and the README names the terms, matching `PackageLicenseExpression` |
| Adoption needed | The README points to `docs/platform-product-adoption.md`, the adoption tool, and `samples/matrix.json` |
| Screenshots expected | Recorded `NOT_APPLICABLE` with the no-UI justification |

## 5. Contract and compatibility

No public API, package identity, package version, or baseline change.
`eng/public-api-baseline.txt` and `eng/package-manifest.json` MUST remain
unchanged and their checks MUST pass. The documented version remains the single
`VersionPrefix` already in `Directory.Build.props`; the `MIT` package license
declaration is reconciled with the new `LICENSE` file. Consumers that already
adopt keep working; the docs become the single entry point.

## 6. Failure and boundary policy

| Case | Result |
|---|---|
| A matrix row names a package that does not exist in `src/` | Review failure; the matrix is checked against `src/` and `eng/package-manifest.json` |
| A usage example does not compile or contradicts a sample | Review failure; fix the example against `samples/` |
| The license file contradicts `PackageLicenseExpression` | Review failure; reconcile before commit |
| The baseline or manifest drifts | `check-public-api.sh` / `generate-package-manifest.sh --check` fail; treat as a regression |
| A screenshot is demanded | `NOT_APPLICABLE` recorded with justification; readiness MUST NOT fail for its absence |

## 7. Verification oracle

- `test -f LICENSE` succeeds and the README references the terms.
- Every package in `src/` appears in the README matrix (checked against
  `eng/package-manifest.json` and `scripts/package-inventory.sh`).
- Each usage example is consistent with `samples/matrix.json` or the starter
  template.
- `./scripts/check-public-api.sh`, `./scripts/generate-package-manifest.sh --check`,
  and `./scripts/quality-gate.sh` pass (no change to `eng/*`).
- `openspec validate --changes --strict --no-interactive` passes.
- `git diff --check` is clean.

## 8. Decision ledger

**Assumptions:** the repository remains a private, independently adoptable
library set, not a shared runtime service; MIT is the intended license (already
declared as `PackageLicenseExpression`); no user-facing UI ships from this
repository.

**Resolved alternatives:**

- (a) Shared readiness manager/common capability vs local docs → **local docs**;
  a manager contradicts the smallest-adoptable-boundary rule and duplicates
  `Platform.Adoption` and workspace-governance.
- (b) Point the README at `docs/packages.md` vs inline the full surface →
  inline an authoritative matrix (kind/dependency/purpose) and link
  `docs/packages.md` for the detailed surface; duplicating every symbol would
  drift.
- (c) Add screenshots vs record N/A → **N/A with justification**; there is no
  user-facing UI.
- (d) Bump `VersionPrefix` for a docs change vs not → **no bump**; docs and
  license add no package change.
- (e) Add a new "readiness" package or tool vs not → **not**; no runtime
  capability is needed.

**Deferred:** a workspace-wide README/readiness template and a cross-repo
adoption dashboard are `workspace-governance` concerns.

**Blockers (not guessed):** none. The MIT choice is already declared in
`Directory.Build.props`; if the owner wants different terms, the `LICENSE` file
and `PackageLicenseExpression` must both change, which is a separate decision.