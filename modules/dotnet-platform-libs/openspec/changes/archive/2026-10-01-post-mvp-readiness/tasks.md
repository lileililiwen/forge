# Tasks: Post-MVP readiness for the .NET platform libraries

## 1. BFS — Baseline and impact coverage

- [x] Inventory `src/` against the README package table and
  `eng/package-manifest.json`; list every package absent from the README.
- [x] Read `docs/packages.md`, `docs/release-governance.md`,
  `docs/dotnet10-migration-contract.md`, `docs/workspace-consumer-bootstrap.md`,
  `docs/platform-product-adoption.md`, and `CHANGELOG.md` to source the
  versioning/consumer guide.
- [x] Inventory `samples/` (`samples/matrix.json`) and `templates/` for
  verifiable usage examples.
- [x] Confirm no public API, manifest, or baseline change is needed; capture the
  `eng/*` hashes.
- [x] Confirm the license decision: `PackageLicenseExpression=MIT` is already
  declared, so no new decision is needed unless the owner changes it.

## 2. DFS — Requirement-by-requirement implementation

- [x] Rewrite the README package section as the authoritative grouped matrix
  (kind, dependency direction, purpose, `docs/packages.md` anchor), checked
  against `src/` and the manifest.
- [x] Add the versioning and consumer guide section (version source, SemVer,
  baseline, source/package mode, central package management, upgrade/rollback,
  links).
- [x] Add minimal usage examples per major concern, consistent with `samples/`
  or the template.
- [x] Add the adoption entry-point section (`docs/platform-product-adoption.md`,
  `tools/Platform.Adoption.Tool`, `samples/matrix.json`).
- [x] Add `LICENSE` (MIT) and the README license section; reconcile with
  `PackageLicenseExpression`.
- [x] Record screenshots as `NOT_APPLICABLE` with the no-UI justification.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify every `src/` package appears exactly once in the matrix and no
  non-existent package is named.
- [x] Verify each example against its sample, template, or `docs/packages.md`.
- [x] Verify `eng/public-api-baseline.txt` and `eng/package-manifest.json` are
  unchanged.
- [x] Confirm no `src/**`, `tests/**`, `Directory.*.props`, or `global.json`
  change.

## 4. Verification

- [x] `dotnet restore Platform.sln && dotnet build Platform.sln -c Release &&
  dotnet test Platform.sln -c Release --nologo`.
- [x] `./scripts/quality-gate.sh`.
- [x] `./scripts/check-public-api.sh` and
  `./scripts/generate-package-manifest.sh --check`.
- [x] `test -f LICENSE` and the README links the terms.
- [x] `openspec validate --changes --strict --no-interactive` and
  `git diff --check`.
- [x] Record actual outputs; report any environment-blocked check with its exact
  next action, never as passing.