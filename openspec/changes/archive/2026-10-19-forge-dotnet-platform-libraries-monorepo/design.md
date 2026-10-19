# Design: forge-dotnet-platform-libraries-monorepo

## Implementation boundary

- Repository: Forge `/home/paul/code/forge`, branch `main`.
- Imported module root: `modules/dotnet-platform-libs/`.
- Source boundary: committed tree at source `main` revision
  `d11fd9b9…`; preserve its complete Git ancestry, including its 38 commits
  ahead of the source's tracked `origin/main`. Exclude untracked files from
  the source import and do not modify the source checkout.
- Preserve the nested module roots and contracts: `Platform.sln`,
  `Directory.Build.props`, `Directory.Packages.props`, `global.json`,
  `NuGet.config`, `.project.json`, `.ai-gate/gate.yaml`, `src/`, `tests/`,
  `ui/`, `templates/`, `eng/`, `scripts/`, `docs/`, `openspec/` and
  `AGENTS.md`.
- Move active workflows to Forge root:
  `.github/workflows/platform-dotnet-quality.yml` and
  `.github/workflows/platform-dotnet-release.yml`. Remove their inert nested
  workflow copies from the module; keep all other project files in the module.
- Add `scripts/check-dotnet-platform-libs-import.py` and
  `docs/dotnet-platform-libs-module.md` to Forge root.
- Update Forge-owned paths in `modules/workspace-governance/projects.json`,
  `modules/workspace-governance/migration/forge-monorepo-path-reconciliation.json`,
  the UI-conformance baseline and generator role mapping, and the UI source/test
  path resolver.
- Do not change Forge Cargo membership, sibling source repositories, external
  product consumers, or Rust platform source in this package.

## Language and runtime

- .NET: SDK `10.0.400` selected by the module-local `global.json`; solution
  target frameworks and package versions remain unchanged. Run commands with
  the current directory at `modules/dotnet-platform-libs/` so the SDK and
  `Directory.Build.props` are resolved from the module.
- UI kit: Node.js/npm runtime as declared in the module's existing
  `package.json` files; no package manager or dependency contract is changed.
- Forge import checker: Python 3 from the Forge root; use only the standard
  library and Git plumbing.
- Forge CI and release orchestration: GitHub Actions at Forge root. CI runs on
  pull requests and main pushes that touch the module or its workflow files.
  The Forge test job fetches full history for the source-ancestry verifier.
  Release remains `workflow_dispatch` with the existing required SemVer input.
- Forge Gate: local Rust binary, scoped to the Forge Git repository root. The
  imported module remains a nested build/test boundary; its project-level
  `.ai-gate/gate.yaml` is retained for portability if exported as a standalone
  repository, while the root path-filtered .NET workflow is the active
  module-specific CI gate in this monorepo.

## Ownership and shared code

The module owns .NET source, tests, templates, UI source, package metadata,
local feeds and native scripts. Forge owns the monorepo root, module discovery,
shared Gate invocation and root GitHub Actions workflow discovery. The
workflow delegates into module scripts rather than reimplementing package
validation.

The module remains outside the root Cargo workspace and retains its own .NET
solution. Existing NuGet IDs, UI package names, versions, licenses and
dependencies do not change. Change the repository/homepage metadata used for
new package source links to the Forge module location. Keep historical
origin URLs in archived specifications as historical evidence.

Workspace Governance remains internal to Forge. Its dotnet project registry
entry resolves to `forge/modules/dotnet-platform-libs` from the portfolio
root. Token source resolution is relative between sibling modules under
`modules/`, not an absolute checkout path. Existing external consumers remain
outside this package and continue to use their declared package/source
coordinates until a later consumer-migration change.

## User experience and interface

No new Forge portal route is introduced. Maintainers use the Forge README,
module README and the native .NET/UI scripts. GitHub Actions keeps two distinct
operator surfaces: path-filtered quality checks and manual versioned release.
CI must not publish. The release's version validation, permissions, concurrency
guard, retained static feed, artifact evidence and Pages deployment remain
visible steps. A failed quality or release stage blocks later steps and
reports through the workflow summary/log. There is no new browser loading,
empty, permission or recovery state because no browser interaction changes.

## Behavioral and data model

| Operation | Input/state | Output | Side effect and invariant |
|---|---|---|---|
| Source import | A local source Git repository and immutable `d11fd9b9…` revision | Ordinary Forge tree at `modules/dotnet-platform-libs/` | Merge ancestry retains source commits; no `.gitmodules`; no source checkout writes |
| Import check | Source revision recorded in import receipt, imported module and Forge Git object database | Exit 0 only for expected source ancestry and declared path/mode/blob differences | Read-only; undeclared/missing/extra files or a missing source commit fail closed |
| CI | Forge PR/main revision and changed module paths | Build, test, API, audit and consumer-conformance job result | Executes only module checks; never pushes, publishes or deploys |
| Manual NuGet release | Explicit version input, Forge revision, repository environment and feed state | Packed packages, retained feed artifact and Pages deploy result | Runs only on explicit dispatch; version gate and existing workflow permissions remain required; failed validation prevents feed mutation |
| Module registry read | Forge portfolio root and project id `dotnet-platform-libs` | Declaration and resolved module path | Path must resolve inside `forge/modules/dotnet-platform-libs`; no fallback to the old sibling checkout |

## Contract and compatibility

- Public NuGet package IDs, package versions, target frameworks, license,
  package dependencies and local consumer configuration remain unchanged.
- UI npm package names/versions and local feed file layout remain unchanged;
  this package does not add registry publication.
- The module's nested `.project.json` id remains `dotnet-platform-libs` and
  its declared verification command remains `dotnet test Platform.sln
  --nologo`. The Forge repository Gate runs at the monorepo root; the root
  module workflow owns module-scoped .NET checks.
- The in-tree project registry preserves that id and changes only its path to
  `forge/modules/dotnet-platform-libs`.
- New package repository metadata resolves to the Forge GitHub repository
  root so SourceLink paths include the actual monorepo-relative file paths;
  `.project.json` links directly to the module directory.
  New static NuGet feed output uses
  `https://lileililiwen.github.io/forge/nuget/`; consumer configuration keeps
  the old endpoint as a fallback, but no legacy feed bytes were available to
  import and endpoint availability is not claimed.
- Root workflow names are unique to avoid coupling to Forge's Rust CI. Release
  workflow tags, secrets, environments and feed path cannot shadow the root
  Forge release or the Rust module's future release contract.

## Failure and boundary policy

- A missing source repository object, unexpected source revision, ancestry
  failure, blob mismatch or unallowlisted adaptation blocks import/archive.
- Missing .NET SDK, Node.js, restore source or package audit tool is reported
  as an environment failure; it is not converted into a successful check.
- A dotnet module declaration that resolves to the origin checkout instead of
  its Forge module path blocks completion.
- Root workflow path filters that fail to include the module or workflow files
  block completion. Nested GitHub workflows are not treated as active checks.
- CI never publishes. Release does not run from `push` or `pull_request`; an
  explicit workflow dispatch and its required version are the only trigger.
- No remote operation is run locally. No package upload is attempted.
- User-owned untracked files in either source checkout remain untouched and
  outside the imported source tree.

## Verification oracle

1. Before import, record source commit `d11fd9b9…`, tracked source tree,
   `origin/main` ancestry, and untracked paths. After import, the importer
   asserts source ancestry, every source-only commit ancestry, tracked path/mode
   equality except declared workflow moves, and blob equality except the
   explicit adaptation allowlist.
2. From the imported module root, run `dotnet restore Platform.sln`,
   `dotnet build Platform.sln -c Release`, `dotnet test Platform.sln -c
   Release`, `scripts/check-public-api.sh`, `scripts/conformance.sh`,
   `scripts/audit-packages.sh`, and `scripts/quality-gate.sh` as required by
   the module scripts and workflow. Run `scripts/test-ui-distribution.sh` and
   the token verifier/self-test. Do not run any publish step.
3. Run the Forge repository Gate dry-run and bounded full Gate from the Forge
   root. Run module-native .NET checks from the module root. Do not claim a
   separate project Gate execution for the nested module: Forge Gate is
   repository-root scoped in a monorepo.
4. Validate root workflow YAML using an installed workflow linter where
   available; otherwise parse/check the file with the repository's available
   YAML validation and record the exact command. Inspect path filters, manual
   release trigger, version gate, and absence of publish steps from CI.
5. Run Workspace Governance module tests that cover project path resolution,
   UI token source selection and the generated UI baseline against the migrated
   project path, with no `dotnet-platform-libs` sibling path.
6. Run Forge name preflight, all strict OpenSpec validation, changed-file
   checks and `git diff --check`; then Forge dry-run and full Gate before
   archive.

## Decision ledger

- **Resolved:** import the complete tracked .NET source repository, including
  its UI kit, because its root quality/conformance suite and
  `Platform.UI.Razor` package bind those files into one source project.
- **Resolved:** retain a nested native solution and module project root; do not
  add these packages to Forge's Cargo workspace. Forge Gate remains
  repository-root scoped; the module workflow supplies module-scoped CI checks
  rather than creating a second nested Git/Gate boundary.
- **Resolved:** move active GitHub workflows to Forge's root because GitHub
  does not discover workflow files in nested module directories.
- **Resolved:** use a Forge-owned Pages feed endpoint for future manual NuGet
  releases and keep the old endpoint as a consumer fallback. No source
  `gh-pages` branch was available to import; consumer availability at either
  URL remains an external release/deployment fact.
- **Deferred:** update sibling consumer NuGet sources and local project
  references in the later consumer portability package.
- **Deferred:** archive the source origin only after consumer migration and
  final origin-retirement checks.
- **Deferred:** import Rust in `forge-rust-platform-libraries-monorepo` after
  this package's required two-commit handoff.
