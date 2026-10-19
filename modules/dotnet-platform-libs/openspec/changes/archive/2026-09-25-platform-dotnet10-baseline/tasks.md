## 1. BFS — Baseline and impact coverage

- [x] 1.1 Record the current worktree, active OpenSpec changes, installed SDKs, configured NuGet sources, platform project count, target-framework count, central package versions, and current build/test/pack baseline without modifying unrelated files.
- [x] 1.2 Build a source-controlled inventory of every platform `src/`, `tests/`, `samples/`, `tools/`, and `templates/` project plus `global.json`, `Directory.Build.*`, `Directory.Packages.props`, package-manifest, sample-matrix, bootstrap, Docker/CI, and documentation references affected by the `net8.0` to `net10.0` baseline.
- [x] 1.3 Record the downstream consumer proposal map from `proposal.md`, including SDK-inherited consumers, local SDK 8 pins, net8-only consumers, mixed consumers, and the nested Git Credential Manager repository; exclude non-C# projects.
- [x] 1.4 Verify the implementation boundary: only `dotnet-platform-libs` files and generated package artifacts are in scope; consumer repositories, application-owned EF migrations, product packages, and unrelated active work remain out of scope for this package.
- [x] 1.5 Confirm the offline-compatible NuGet source/cache contains the selected SDK-compatible ASP.NET Core, EF Core, and Microsoft.Extensions 10.x package versions; record exact versions and mark the change blocked if any mandatory asset is unavailable.
- [x] 1.6 Add or update verification skeletons that fail when source-controlled platform project metadata, bootstrap conditions, sample metadata, package manifests, or current technical docs retain SDK 8, `net8.0`, or Microsoft 8.x baseline instructions.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Update `global.json` to SDK `10.0.400`, `rollForward: latestPatch`, and `allowPrerelease: false`, then verify repository-local SDK resolution reports the declared stable SDK.
- [x] 2.2 Remove every repository-owned SDK 8 selector, `net8.0` target, and current SDK 8/.NET 8 instruction before accepting any compatibility result; do not add a dual-target or fallback path.
- [x] 2.3 Change all platform-owned production, adapter, testing, test, sample, tool, and template project targets to `net10.0`, including any target defaults or conditions in repository MSBuild files.
- [x] 2.3 Update `build/Platform.Consumer.props`, workspace-consumer eligibility checks, and related tests so the local platform reference applies to compatible `net10.0` consumers and explicit opt-out behavior remains unchanged.
- [x] 2.4 Update `Directory.Packages.props` and any explicit platform-owned Microsoft package references to the selected compatible 10.x versions; preserve third-party versions unless a documented restore or compile failure proves an update necessary.
- [x] 2.5 Update the application template, template tests, sample projects, `samples/matrix.json`, adoption fixtures, and generated-project assertions to produce and validate `net10.0` applications.
- [x] 2.6 Regenerate `eng/package-manifest.json` with `scripts/generate-package-manifest.sh` and make its check fail on source/manifest target-framework or dependency drift.
- [x] 2.7 Update current baseline documentation and commands to describe SDK `10.0.400` and `net10.0`; preserve historical HANDOFF evidence as historical rather than rewriting prior SDK 8 verification claims.
- [x] 2.8 Add focused tests for SDK/target/package metadata, framework-neutral dependency direction, test-only package boundaries, template output, consumer bootstrap evaluation, and package manifest consistency.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Search all source-controlled repository files, excluding generated output and historical archives, for unintended SDK 8 pins, `net8.0` target declarations, Microsoft 8.x framework package references, and .NET 8 runtime/tooling image references.
- [x] 3.2 Verify every framework-neutral package remains free of ASP.NET Core and EF Core production dependencies after package updates, and verify every testing package remains excluded from production projects.
- [x] 3.3 Restore and evaluate the complete platform solution and generated template/sample matrix under SDK `10.0.400`; include at least one framework-neutral package, one ASP.NET Core adapter, one EF Core adapter, one test-only package, and the template output.
- [x] 3.4 Regenerate or validate EF migration snapshots only through EF tooling 10 where applicable; do not treat textual `ProductVersion` replacement as migration evidence.
- [x] 3.5 Review package/API diffs for unintended public API, package ownership, dependency-direction, serialization, or compatibility changes and record each intentional breaking baseline change.
- [x] 3.6 Write the downstream migration contract listing required SDK, target framework, Microsoft package, Docker/CI, EF tooling, offline restore, and repository-native verification changes for each consumer migration package.

## 4. Verification

- [x] 4.1 Run `dotnet restore Platform.sln --ignore-failed-sources -p:NuGetAudit=false --nologo -m:1` under SDK `10.0.400` and record package-source/cache evidence.
- [x] 4.2 Run `dotnet build Platform.sln -c Release --no-restore --nologo -m:1` and require zero errors; record warnings separately and do not hide new warnings.
- [x] 4.3 Run `dotnet test Platform.sln -c Release --no-build --no-restore --nologo -m:1`, focused metadata/bootstrap/template/architecture tests, and the generated consumer smoke test.
- [x] 4.4 Run `dotnet pack Platform.sln -c Release --no-build --no-restore --nologo -m:1` and inspect representative package dependency groups and target frameworks; specifically verify the known SDK 10 `_GetFrameworkAssemblyReferences` packaging path.
- [x] 4.5 Run `scripts/generate-package-manifest.sh --check`, repository quality gates, scoped `dotnet format --verify-no-changes`, `git diff --check`, and `openspec validate --changes --strict --no-interactive`.
- [x] 4.6 Record PASS, FAIL, BLOCKED, or NOT_APPLICABLE evidence for every mandatory gate, update this task list only from command output, and leave the change active if any required check is blocked or failed.
