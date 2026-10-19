# Design: forge-rust-platform-libraries-monorepo

## 1. Implementation boundary

Import only committed source from `/home/paul/code/rust-platform-libs` at
`4762746e35b3886b21e2c87bf602aed14bc9dea8` into
`modules/rust-platform-libs/`. The source Git worktree is clean, has 161
tracked paths and 30 commits, and has no configured remote. Keep Forge's root
Cargo workspace and source `Cargo.toml` as separate workspaces. Do not add a
Git submodule. Preserve every tracked source path/mode/blob except the explicit
adaptations below; do not copy ignored `target/`, `Cargo.lock`, editor state or
other local artifacts.

Forge-root changes are limited to:

- `.github/workflows/platform-rust-quality.yml` and
  `.github/workflows/platform-rust-publish.yml`;
- `modules/workspace-governance/projects.json` and
  `modules/workspace-governance/migration/forge-monorepo-path-reconciliation.json`;
- root import receipt/checker, its focused tests, Forge README/module guide,
  OpenSpec archive/canonical specs and roadmap/handoff evidence.

Within the module, adapt `.project.json`, `.ai-gate/gate.yaml`, Cargo
repository/homepage metadata, the two contract-parity test path resolvers,
module documentation and source workflow removal. Do not edit sibling product
repositories, Forge root Cargo dependencies, crate APIs, versions, feature
sets, `rust-toolchain.toml` or the license.

## 2. Language and runtime

- Rust edition 2021; `rust-toolchain.toml` pins Rust 1.95.0 with `rustfmt` and
  `clippy`; workspace `rust-version` remains 1.75.
- Keep the source workspace's eight crates and its `tests/consumer-fixture`
  as members. Cargo commands execute with current directory
  `modules/rust-platform-libs/`; no root Cargo `--manifest-path` or root
  workspace membership is added.
- Native verification commands are:
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo deny check`, `cargo build --workspace`,
  `cargo test --workspace` and `./scripts/conformance.sh`.
- Root quality CI installs Rust 1.95.0 explicitly, verifies that version
  against the module toolchain file, installs `cargo-deny`, and runs those
  commands in a module-scoped working directory. The Forge root Gate is a
  separate check over the Forge Git repository.
- Release workflow jobs also use the Rust 1.95.0 module toolchain and run the
  same quality preconditions before any `cargo publish` command.

## 3. Ownership and shared code

Forge owns the in-tree module and the language-neutral wire contracts under
`contracts/`. The Rust `platform-contracts` crate continues to own Rust payload
types and validation behavior. Forge currently owns six schema families and
explicitly marks identity-subject, job-outcome, permission-decision and
tenant-context unsupported. Do not copy those unsupported schemas into Forge.
The two integration-test resolvers SHALL use `PLATFORM_CONTRACTS_DIR` when set
and otherwise derive `<forge-root>/contracts` from `CARGO_MANIFEST_DIR`.

When the selected root contains Forge's `registry.json` and
`family-support.json`, load its `contracts` object with
`ContractRegistry::from_value`, resolve its schema paths relative to that root,
validate all six supported fixture pairs, and assert the four unsupported
statuses and reasons. When an explicit override instead contains the legacy
`schemas/registry.json` layout, retain the source tests' nine-family manifest
walk. If the selected root matches neither layout, fail without searching a
sibling or silently reducing coverage. Do not duplicate schemas inside the
Rust module.

Forge Governance owns the internal registry and project-path reconciliation.
Keep project ID `rust-platform-libs`, Rust profile `rust-platform`, role and
capability declaration. Change only its in-tree path from `rust-platform-libs`
to `forge/modules/rust-platform-libs`, preserving the wider workspace registry
root convention. Update module homepage/repository URLs to the Forge repository
and module page respectively. Crate `documentation` URLs remain their docs.rs
package URLs. There is no new shared crate, source-level sibling dependency or
consumer migration.

The module `.project.json` and `.ai-gate/gate.yaml` SHALL declare `forge` as
their Gate runtime. The Forge root CLI and root Gate own repository-level
quality evidence; this module declaration does not create a second Git
boundary. The existing Forge-owned Governance quality command remains rooted
at the workspace and selects project ID `rust-platform-libs`.

## 4. User experience and interface

There is no human-facing product screen or browser journey. Maintainers use the
Forge README, module README and native Cargo commands. Release operators use
the GitHub Actions workflow named `Rust platform crates: publish`; its
precondition and outcome are visible in the workflow summary. The existing
GitHub Actions UI is the operator surface and gains a distinct workflow name
and namespaced tag; no Forge route, page or command is introduced.

The quality workflow exposes queued/running/success/failed states through
GitHub Actions and performs no writes. The publish workflow reports missing
approval, token, invalid version/tag, failed validation, partial publish or
successful per-crate publication as explicit job outcomes; a partially
published rerun retains the source workflow's existing idempotent
already-published-version behavior. No browser/device or accessibility
verification applies because no UI is added or changed.

## 5. Behavioral and data model

### Source import

| Input | State transition | Invariant |
|---|---|---|
| Clean source commit and its Git object graph | Import source tree under module prefix and make the source HEAD a parent of the Forge import commit | All 30 source commits remain ancestors; source tracked path/mode/blob set matches the module except enumerated adaptations |
| Source working tree has uncommitted or untracked files | No import of those files | No source-local state is copied or modified |
| Source revision or ancestry is unavailable | Import checker fails | No fabricated provenance or silent current-branch substitution |
| Imported source path or content drifts without an adaptation declaration | Import checker fails | Every difference is attributable to a named adaptation |

The source checkout is read-only for this change. The import receipt names the
source revision, source file count, commit count and adaptation set. The
checker obtains the source snapshot from the recorded source commit preserved
in Forge ancestry, rather than a live sibling path, so future checks remain
portable.

### CI and release

CI runs for pushes and pull requests affecting
`modules/rust-platform-libs/**` or the Rust quality workflow, and by manual
dispatch. It is read-only and cannot call the publisher. The publisher accepts
only tags named `rust-platform-libs-v<SemVer>` or an explicit workflow dispatch
with a SemVer `version` and `confirm_owner_approval=approve`. Both paths require
the tracked `modules/rust-platform-libs/PUBLICATION-APPROVED` marker, the
`CARGO_REGISTRY_TOKEN` repository secret, exact equality to the module
workspace version and successful release verification before publishing.
Each crate is published from the module directory in `.publish-order`; the
workflow preserves name/version preflight, propagation polling and skip
existing-version behavior. It SHALL NOT run on untagged Forge push/PR events.

The tag is module-namespaced because Forge uses one shared Git tag namespace.
The crate version and crates.io package IDs do not change. Publication remains
blocked while any owner precondition is absent. No marker, secret, tag or
release is created by this change.

### Contract path resolution

For each parity test, resolution order is:

1. If `PLATFORM_CONTRACTS_DIR` is set, use exactly that path.
2. Otherwise resolve `<forge-root>/contracts` from the nested module location.
3. Detect Forge layout by the presence of `registry.json` and
   `family-support.json`; detect legacy layout by `schemas/registry.json`.
4. If the selected root matches neither layout or a required schema/fixture is
   missing, fail with the resolved path and an actionable message; never skip
   parity tests or treat missing schemas as a pass.

The expected repository layout is
`<forge-root>/modules/rust-platform-libs/crates/platform-contracts/` and
`<forge-root>/contracts/registry.json`, `family-support.json`, six
`schemas/*.schema.json` files and `fixtures/<family>/{valid,invalid}.json`
pairs. The Forge-layout test asserts all six supported families and confirms
that identity-subject, job-outcome, permission-decision and tenant-context
remain unsupported. The legacy override continues validating its nine-family
manifest matrix.

## 6. Contract and compatibility

- Preserve crate IDs, Rust API, version `0.1.0`, package metadata except
  repository/homepage URLs, license `Apache-2.0`, MSRV declaration, feature
  defaults, dependency policy and `.publish-order`.
- Preserve `PLATFORM_CONTRACTS_DIR` as a filesystem path override. Its default
  changes from a sibling checkout to Forge's in-tree canonical contracts.
- Preserve the owner approval marker, secret name, exact-version validation,
  publish sequence, idempotent published-version skip, and crates.io
  destination. The Git tag wire format changes from generic `vX.Y.Z` to
  `rust-platform-libs-vX.Y.Z`; do not treat old Forge tags as module releases.
- Root workflow names must be unique and explicit; nested `.github/workflows`
  source files are removed from the imported module because GitHub only loads
  workflows from the repository root.
- Forge Governance keeps project ID `rust-platform-libs` and changes the path
  to `forge/modules/rust-platform-libs` according to its workspace-root
  convention. Registry consumers outside Forge are not edited.

## 7. Failure and boundary policy

| Condition | Required result |
|---|---|
| Source commit or an ancestor is missing | Import checker fails; do not claim history preservation |
| Untracked or ignored source files exist | Preserve source checkout and exclude them from the import; report their presence |
| Extra, missing or changed imported path is not allowlisted | Import checker fails |
| Canonical `contracts/` or required schema is missing | Cargo parity tests fail with a resolved path and next action |
| `PLATFORM_CONTRACTS_DIR` points to a missing or malformed contract root | Parity tests fail; do not fall back to another path |
| Module quality check or conformance fails | CI is red; no publication command is reachable |
| Release tag is malformed, version differs, marker/secret missing, or confirmation is absent | Workflow refuses before publication |
| A crate version already exists after partial release | Preserve the existing idempotent skip behavior for that version; continue only under the source workflow's established rules |
| GitHub Actions cannot access the registry or dependency source | Job fails and reports the failed step; no fabricated publication evidence |
| New UI is requested implicitly by this source move | Not applicable; no human-facing Forge UI capability is introduced |

## 8. Verification oracle

Before archive, run and record:

1. `python3 scripts/check-rust-platform-libs-import.py` and
   `python3 scripts/tests/test_rust_platform_libraries_import.py`; assert the
   recorded source revision, all source ancestry, exact tracked tree
   accounting and refusal on undeclared drift.
2. From `modules/rust-platform-libs/`, run the exact native format, Clippy,
   deny, build, workspace test and conformance commands in section 2. The
   test run must exercise Forge's `contracts/` default without a sibling path.
3. Run affected Workspace Governance path/registry tests and the module
   declaration audit; verify `rust-platform-libs` resolves to
   `forge/modules/rust-platform-libs`.
4. Parse both new workflow YAML files; assert module path filters,
   toolchain version, read-only CI, distinct release tag namespace, marker,
   token, SemVer checks and no publish step in quality CI.
5. Run Forge Gate dry-run and bounded full Gate at repository root; do not
   treat module-native Cargo checks as a substitute for root Gate.
6. Run name preflight, `openspec validate --all --strict --no-interactive`,
   relevant status/instructions, `git diff --check` and staged-diff review.

The source project has no configured Git remote, so there is no remote
ancestry baseline to compare against. The import oracle is all 30 commits
reachable from source HEAD, verified as ancestors after import, plus the exact
tree at that immutable HEAD. This is explicitly reported rather than
inventing a source upstream comparison.

## 9. Decision ledger

- **Resolved:** Import the whole Rust workspace under `modules/`, not into the
  Forge root Cargo workspace; the nested workspace and version graph are its
  native build boundary.
- **Resolved:** Preserve all 30 source commits through a local Git object
  import; there is no source remote or origin branch to retain.
- **Resolved:** Forge `contracts/` is authoritative; keep the Rust crate as
  consumer/validator and retain its explicit path override.
- **Resolved:** Use `rust-platform-libs-vX.Y.Z` tags to isolate module release
  versioning from Forge tags while retaining current owner gates.
- **Resolved:** Root Forge Gate evaluates the repository; module workflows
  execute native Cargo checks from the module directory. The module is not a
  second Git repository after import.
- **Deferred:** Consumer migration and source-origin archival are separate
  roadmap changes after both platform modules are imported.
- **Deferred:** The actual first crates.io release remains blocked on owner
  approval and publisher credentials; no release outcome is claimed here.
- **Blockers:** None for the source import. Missing Cargo dependencies or
  missing local `cargo-deny` are environment verification blockers only and
  must be reported with exact command and remedy if encountered.
