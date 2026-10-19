## 1. BFS — Baseline and impact coverage

- [x] 1.1 Verify source HEAD `4762746e35b3886b21e2c87bf602aed14bc9dea8`, clean source worktree, 161 tracked files, 30 commits and no remote; inventory ignored/untracked artifacts without changing them.
- [x] 1.2 Map all eight crate manifests, workspace/toolchain/license/version configuration, native CI/publish workflows, contract test resolution, Governance registry consumers and module path reconciliation.
- [x] 1.3 Add failing import checker tests for source revision/ancestry, exact tracked paths/modes/blobs, declared adaptations, absent sibling checkout, absent Git submodule and ignored/untracked source exclusion.
- [x] 1.4 Confirm the Cargo workspace remains nested; document crate, tag, marker, secret, publisher and canonical contract path invariants before import.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Import source commit `4762746e35b3886b21e2c87bf602aed14bc9dea8` as ordinary `modules/rust-platform-libs/` content and preserve all source commits in Forge ancestry.
- [x] 2.2 Add source receipt and deterministic import verifier comparing the source commit's tracked paths/modes/blobs, ancestry and an explicit allowlist of workflow/metadata/contract path adaptations.
- [x] 2.3 Preserve the independent Cargo workspace, eight crate identities, version/feature/MSRV/license/publish-order policy, consumer fixture and all module-local build and test scripts.
- [x] 2.4 Adapt the two Rust parity suites to load Forge's six supported registry/fixture families and assert the four explicitly unsupported statuses; preserve the legacy nine-family `PLATFORM_CONTRACTS_DIR` layout and fail closed on malformed inputs.
- [x] 2.5 Relocate Rust quality and publish workflows to unique Forge-root workflows with path filters, Rust 1.95.0, module working directories and no publish path in quality CI.
- [x] 2.6 Namespace Rust release tags; preserve approval marker, token, version equality, explicit workflow dispatch approval, preflight and idempotent publication behavior; never publish during this change.
- [x] 2.7 Update project metadata and Forge-owned Governance registry/path reconciliation to resolve `rust-platform-libs` at `forge/modules/rust-platform-libs`; declare Forge as the module Gate runtime.
- [x] 2.8 Document source provenance, module commands, canonical contracts, workflow names, tag shape and unchanged publication preconditions.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify no source untracked/ignored files, Cargo build outputs, source-local lockfile, or `.gitmodules` were imported; verify all 30 source commits remain ancestors.
- [x] 3.2 Run module format, Clippy, deny, build, workspace tests and consumer conformance using the pinned toolchain and Forge-owned six-family contract data with no sibling fallback.
- [x] 3.3 Verify Governance registration and path-map resolution, project declaration audit, Forge Gate metadata and root Gate/module Cargo boundary.
- [x] 3.4 Verify quality workflow path filters and all native commands; verify it cannot publish; test publisher refusal for absent marker/token/approval, malformed tag and version mismatch.
- [x] 3.5 Reconcile crate/API/package compatibility, contract fixture coverage, failure diagnostics, source provenance, release permission boundary and every proposal impact row.

## 4. Verification

- [x] 4.1 Run focused import tests, all Rust native workspace checks, affected Governance tests, workflow parsing/contract checks and changed-file formatting.
- [x] 4.2 Run name preflight, selected status/instructions, all strict OpenSpec validation and `git diff --check`.
- [x] 4.3 Run `forge gate --dry-run` and bounded full Forge Gate at repository root; record the actual verdict and evidence in HANDOFF.
- [x] 4.4 Promote canonical requirements and archive only after import, native module, workflow and release-refusal checks pass.
- [x] 4.5 Commit implementation/tests/archive, advance HANDOFF to the next eligible roadmap change, then commit HANDOFF alone.
