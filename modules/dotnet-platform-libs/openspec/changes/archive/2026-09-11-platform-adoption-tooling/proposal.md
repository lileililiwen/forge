## Why

Independent subprojects need a fast way to discover whether their SDK, package versions, project references, and local services are compatible with the platform. The starter kit's `doctor` and update workflows demonstrate the need, but a platform consumer tool must remain small and must target an explicit project directory.

## What Changes

- Add a small adoption CLI or script surface with `doctor`, inventory, and conformance commands.
- Require an explicit project/repository directory and produce machine-readable output as well as human-readable summaries.
- Support preview-only package alignment and upgrade/rollback checks without editing application source automatically.

## Capabilities

### New Capabilities

- `adoption-tooling`: explicit-directory diagnostics and package adoption checks.

### Modified Capabilities

- None.

## Impact

Adds a tooling project/script and test fixtures; no production package dependency. It may consume the existing package manifest and consumer conformance conventions.

## Non-Goals

- No workspace-wide rewrite.
- No automatic package upgrades or commits.
- No assumption that all consumers use the same solution, database, frontend, or deployment stack.
