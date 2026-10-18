# Design: hide governance internals behind Forge

## Implementation boundary

- Forge Rust CLI: add `ConcernCommands::Check`, its command-catalog entry and
  early exit-code-owning dispatch under `src/cli/`, `src/api/command_catalog/`
  and `src/main.rs`.
- Canonical checker source: keep
  `modules/workspace-governance/scripts/check_concern.py` as the single
  implementation. The facade invokes it from the Forge source tree or the
  installed resource bundle; product projects never resolve that path. The
  product quality IDs use `product_code_quality.py` plus its `vocabulary.py`
  and `vocabulary.json` data.
- Distribution: add the checker to `share/forge/governance/` through
  `scripts/package.sh` and `scripts/install.sh`; verify it in `scripts/smoke.sh`.
- In-tree consumers: update the live Gate manifests and generic concern
  wrappers in `crates/driftwatchdog`, `crates/cargo-audit-boost`,
  `crates/cargo-matrix-boost`, and `crates/cargo-sonar`. Update current README
  usage instructions and contract-sync source. Preserve archived OpenSpec
  records as historical evidence.
- Remove the Forge-owner portfolio audit from concrete crate Gate manifests;
  Forge's internal governance module keeps its own `--self-check` and CI.
  The product Gate baseline remains build/test/repository/security plus
  explicitly declared product checks.
- Do not modify external sibling checkouts, their OpenSpecs, their CI, or
  remote state in this change.

## Language and runtime

Rust stable, edition 2021, for the public CLI and package/install plumbing.
Python 3 for the internal checker, using only its existing standard-library
dependencies. The checker is read-only and emits its existing JSON document.
During development the resource is resolved relative to
`env!("CARGO_MANIFEST_DIR")`; installed binaries resolve it relative to the
executable prefix at `share/forge/governance/check_concern.py`.

## Ownership and shared code

Forge owns the public command, resource path and release lifecycle. The
Workspace Governance module remains the canonical source of the generic
checker during this migration, but projects call only Forge. Avoid copying the
checker into each crate. In-tree wrapper scripts may remain as tiny
compatibility entry points, but they invoke `forge concern check` and contain
no source-root calculation or Python checker implementation.

Forge's generic concern IDs are the checker IDs accepted by `check_concern.py`;
the two product-quality IDs `placeholder-threshold` and
`product-code-boundary` route to the product quality checker. The facade
validates the requested ID against this closed set before starting Python.
`forge concern check <id> [project-root]` defaults the root to
`.`. It inherits standard input/output and returns the checker exit status
unchanged. The current checker emits JSON on PASS/FAIL/NOT_APPLICABLE and a
bounded stderr diagnostic on REVIEW_REQUIRED.

## User experience and interface

Usage:

```text
forge concern check <accessibility|fixtures|placeholder-threshold|product-code-boundary|privacy|provenance|schemas> [PROJECT_ROOT]
```

`forge concern --help` lists the IDs and states that the check is read-only.
The project root defaults to the current directory. Unknown IDs and invalid
roots fail before the helper starts. Missing Python/resource failures name
the exact Forge requirement. No new state, credential or registry write is
performed.

## Behavioral model

1. Parse the top-level `concern` command and closed subcommand.
2. Resolve the bundled helper. In the source checkout, use the canonical
   module script; otherwise use the installed resource. An invalid explicit
   resource is never silently replaced (there is no user override in this
   version).
3. Validate root is an existing directory and concern is supported.
4. Run `python3 <helper> <concern> <canonical-project-root>` with inherited
   stdio and no shell.
5. Return the helper's status exactly. Spawn failure returns a typed Forge
   error and nonzero exit.
6. The checker may return 0 for PASS/NOT_APPLICABLE, 1 for findings, or 2 for
   incomplete coverage; Forge does not remap these outcomes.

The product Gate manifests remove the workspace-wide
`governance-quality` binding. That check was running the Forge owner's
portfolio registry from each product Gate and depended on its sibling
layout. Forge's Workspace Governance CI/self-check remains the owner-side
oracle. Other source/build/test/security/quality checks remain in the product
Gate and are not dropped.

## Contract and compatibility

- CLI is `forge concern check <id> [project-root]`; `forge check` remains
  untouched.
- Concern IDs are closed to the seven supported IDs: five generic governance
  checks and the two product-quality checks.
- Existing checker JSON `schema_version: 1`, finding shape, status values,
  stdout/stderr channels and exit codes remain byte-semantically compatible.
- Resource location is stable within Forge release archives under
  `share/forge/governance/`, with vocabulary data at `share/forge/vocabulary.json`.
- Concrete in-tree Gate manifests use `rule_pack: forge-baseline/1.0.0` and
  do not define `governance-quality` commands or provider names.
- Workspace Governance's own project metadata, registry, schemas and source
  stay in-tree and are not renamed in this change.

## Failure and boundary policy

- Unknown concern: typed argument error, no subprocess.
- Missing/non-directory project root: typed argument error, no subprocess.
- Missing helper or Python: clear Forge error, nonzero.
- Checker operational exit 2: preserved as incomplete, never PASS.
- Checker findings exit 1: preserved with JSON evidence.
- Symlinks, read-only checks, no network and no registry mutation follow the
  existing Python checker implementation.
- Existing native Gate commands remain mandatory. Removing the
  workspace-wide audit from product-local Gate does not delete Workspace
  Governance's own CI/self-check.

## Verification oracle

- Unit/CLI contracts cover help, every supported concern, unknown ID, invalid
  root, resource missing, spawn failure, exact stdout/stderr/status forwarding,
  and source-vs-installed resource resolution.
- Release package/install/smoke confirms the resource exists and a concern
  check runs after install.
- Forge source audit confirms consolidated product configuration and scripts
  contain no active path/provider identity reference; archived OpenSpecs and
  maintainer history are excluded from this compatibility scan.
- Each consolidated crate retains its declared native build/test/security
  commands and no longer binds the workspace-wide audit as a product Gate
  check. Local `driftwatchdog gate --dry-run` resolves no missing command.
- Workspace Governance self-check and its declared unit suite remain runnable
  inside the module.
- Root Forge Gate dry-run/full run, name preflight, strict OpenSpec validation,
  import verifier (with explicit post-import maintenance entries), and diff
  checks complete before archive.

## Decision ledger

- Use the current checker as the canonical implementation and expose it via a
  Forge CLI facade. Do not duplicate its logic into each project.
- Use the existing resource packaging pattern from runtime templates. Keep
  `scripts/package.sh`, `scripts/install.sh` and `scripts/smoke.sh` aligned.
- Portfolio registry checks stay in Forge's governance owner. Running a
  workspace-wide registry audit from every product Gate is the coupling being
  removed.
- External sibling repos migrate only after the CLI/resource contract is
  released and their own HANDOFF/Git state has been checked.
- Python source-size conformance remains a separate mandatory package before
  origin retirement and does not block this interface migration.
