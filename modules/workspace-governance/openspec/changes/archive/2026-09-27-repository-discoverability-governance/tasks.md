# Tasks: Repository Discoverability Governance

## 1. BFS — Baseline and impact coverage

- [x] Map declaration schema, profile requirements, discovery flow, initializer, GitHub subprocess boundary, and existing tests.
- [x] Add failing tests for local metadata validation, license findings, additive repair, authentication blocking, and GitHub command construction.
- [x] Confirm the package uses `workspace-governance` ownership and does not modify sibling repositories.

## 2. DFS — Requirement-by-requirement implementation

- [x] Implement metadata validation and schema/profile requirements.
- [x] Implement read-only audit findings and JSON output compatibility.
- [x] Implement additive local metadata and license repair.
- [x] Implement authenticated GitHub check, metadata edit, and explicit repository creation/push path.
- [x] Document commands, boundaries, and evidence states in the skill/reference/README surfaces.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify inventory exceptions and existing declarations remain compatible.
- [x] Verify audit mode does not mutate files, remotes, or GitHub.
- [x] Verify existing topics/remotes/licenses are preserved and secret values never enter output.
- [x] Verify all stable findings map to tests and all tests map to a requirement.

## 4. Verification

- [x] Run focused tests after each implementation slice.
- [x] Run the full unit suite and all repository self-checks.
- [x] Run strict OpenSpec validation and inspect the final diff/status.
- [x] Archive only after all required evidence is present.
