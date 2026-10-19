## 1. BFS — Baseline and impact coverage

- [x] 1.1 Inventory the source project tree, native build/runtime roots, publication workflows, root registry consumers and Forge import/Gate patterns.
- [x] 1.2 Record source HEAD `d11fd9b`, 38 commits ahead of origin, 1,604 tracked files, 79 .NET library projects, 70 test projects, six UI package manifests, pinned .NET 10.0.400 and the two untracked source files to preserve.
- [x] 1.3 Add failing import provenance/path-resolution tests before moving source; cover source ancestry, exact tracked tree, declared adaptations and no sibling fallback. The contract tests were run red before the checker and import were added, then pass against the implemented import.
- [x] 1.4 Verify the package map keeps the Rust workspace in a separate follow-up with its independent toolchain and publisher.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Import committed source at `d11fd9b` as ordinary `modules/dotnet-platform-libs/` content and preserve its complete source commit ancestry without a Git submodule.
- [x] 2.2 Add an import receipt and deterministic verifier for source revision, source-only commit ancestry, tracked file paths/modes/blobs and intentional adaptation allowlists.
- [x] 2.3 Keep the native .NET solution, module declaration, UI packages, local feeds, tests and package identities independently runnable from the module root; retain its Gate declaration without treating the nested module as a separate Git root.
- [x] 2.4 Move .NET quality and manual release workflows into unique Forge-root workflows with module path filters and module working directories; preserve release preconditions and prohibit automatic publication.
- [x] 2.5 Retarget new .NET package/source links and future static-feed output to Forge, preserve existing package coordinates and retain the legacy feed URL as a fallback; record that no old feed branch was available to copy.
- [x] 2.6 Update Forge-owned project registry, monorepo path reconciliation and UI token checks to resolve the in-tree module without absolute sibling checkout paths.
- [x] 2.7 Document source provenance, origin/consumer boundary, local module commands and CI/release ownership in Forge.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify import tree parity and history preservation; assert no `.gitmodules`, no copied untracked source files and no undeclared drift.
- [x] 3.2 Run .NET solution quality/build/test, package API check, package audit, packed-consumer conformance, UI token verifier/self-test and UI distribution suite from the in-tree module.
- [x] 3.3 Verify the Governance registry resolves `dotnet-platform-libs` to `modules/dotnet-platform-libs`; confirm Forge Gate remains repository-root scoped and module checks run from the module root without origin sibling resolution.
- [x] 3.4 Verify root CI filters select every module source/workflow change and never invoke the release workflow; verify manual release retains version, permission and feed checks.
- [x] 3.5 Reconcile every requirement, consumer, publication boundary, failed-check path, local source untracked file and UI applicability decision.

## 4. Verification

- [x] 4.1 Run affected tests, module verification and changed-file formatting; record unavailable tools exactly.
- [x] 4.2 Run name preflight, active status/instructions, all strict OpenSpec validation and `git diff --check`.
- [x] 4.3 Run Forge Gate dry-run and bounded full Gate from the Forge root; run module-native .NET checks from its root and record both verdicts in HANDOFF.
- [x] 4.4 Promote canonical requirements and archive only after provenance, native module, CI and release checks pass.
- [x] 4.5 Commit implementation/tests/archive, advance HANDOFF to `forge-rust-platform-libraries-monorepo`, then commit HANDOFF alone.
