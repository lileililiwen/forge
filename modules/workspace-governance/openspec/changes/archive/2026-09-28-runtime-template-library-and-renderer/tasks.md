# Tasks: Add reusable runtime Docker and Compose templates

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Inventory current profiles, declaration schema, bootstrap script,
  deployment manifest, Jenkins consumer, DriftWatchdog contract, and concrete
  product deployment fixtures.
- [x] 1.2 Define the six type contracts, variables, generated files, and
  unsupported/ambiguous boundaries in `template.json` fixtures.
- [x] 1.3 Add renderer and metadata test skeletons before implementation.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add versioned runtime template registry and six template families.
- [x] 2.2 Implement `list`, `inspect`, `render`, and `check` with confined,
  atomic, no-secret rendering and exact overwrite policy.
- [x] 2.3 Extend project declaration schema and additive metadata update logic.
- [x] 2.4 Validate the relative Compose handoff in workspace self-check while
  preserving the existing Jenkins-manifest consumer contract.
- [x] 2.5 Document bootstrap usage and template maintenance/versioning.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Test all six templates against fixture repositories and verify no
  unresolved placeholders, absolute paths, or secrets.
- [x] 3.2 Test ambiguous detection, missing variables, overwrite refusal,
  declaration preservation, tool-unavailable, and malformed Compose paths.
- [x] 3.3 Re-audit workspace profiles, `init_project.py`, Jenkins manifest,
  registry metadata, and DriftWatchdog contract compatibility.
- [x] 3.4 Confirm no product repository or sibling deployment behavior changes
  in this package.

## 4. Verification

- [x] 4.1 Run Python unit tests, workspace self-check, Jenkins manifest check,
  strict OpenSpec validation, and `git diff --check`.
- [x] 4.2 Record whether Docker Compose validation ran or was unavailable;
  never mark configuration as runtime verified without it.
