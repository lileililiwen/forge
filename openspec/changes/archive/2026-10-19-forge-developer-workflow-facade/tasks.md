## 1. BFS — Baseline and impact coverage

- [x] 1.1 Inventory current module declarations, executable-relative workspace resolution, Gate options/output/persistence, command catalog derivation, existing Gate/use commands, and test seams.
- [x] 1.2 Add failing resolver tests for sorted discovery, ignored undeclared directories, malformed declarations, duplicate IDs, missing Gate declarations and canonical path escapes.
- [x] 1.3 Record module source checkout boundary, no sibling/CWD fallback, no new persistence, and existing Gate ownership for the full/fast/dry-run paths.
- [x] 1.4 Verify the one package covers catalog discovery and ID-based Gate invocation without changing hosted CI, package release, deployment or browser command execution.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add validated immediate-child module catalog resolving `.project.json`, Gate declarations, IDs, stack/profile and canonical module paths.
- [x] 2.2 Add `forge module list` with sorted human/JSON output and explicit empty, unavailable and malformed states.
- [x] 2.3 Add `forge module verify <ID>` with exact lookup and one delegation to the existing Gate handler, preserving profile, dry-run, timeout, evidence and exit behavior.
- [x] 2.4 Register the CLI help surface, preserve `forge gate` and `forge use`, and keep web command catalog classification CLI-only.
- [x] 2.5 Document source-checkout module discovery, native module ownership, Gate evidence path, failures and remote-operation boundary.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify module metadata cannot escape `modules/`, unknown IDs cannot become paths, duplicate identities fail before execution, and no shell/network/sibling fallback was added.
- [x] 3.2 Verify human/JSON list consistency, gate dry-run non-persistence, full Gate failure propagation and existing CLI help/catalog behavior.
- [x] 3.3 Reconcile module IDs and Gate manifests for Workspace Governance, .NET platform libraries and Rust platform libraries; verify absent/unsupported declarations fail clearly.
- [x] 3.4 Review proposal impact map, command UX/error states, installed-binary limitation, and separation from CI, publish and deploy owners.

## 4. Verification

- [x] 4.1 Run focused module resolver/CLI contract tests, relevant web command-catalog tests and Rust format checks.
- [x] 4.2 Run `scripts/check-tests.sh`, name preflight, selected status/instructions, strict OpenSpec validation and `git diff --check`.
- [x] 4.3 Run Forge Gate dry-run and bounded full Gate from the repository root; record the verdict in HANDOFF.
- [x] 4.4 Archive without `--skip-specs`, promote canonical requirements, and verify the archive.
- [x] 4.5 Commit implementation/archive, update HANDOFF to the next eligible change, then commit HANDOFF alone.
