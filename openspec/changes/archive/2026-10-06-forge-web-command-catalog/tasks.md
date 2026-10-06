# Tasks: forge-web-command-catalog

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Enumerate every top-level and nested Clap subcommand and map current Core/API implementation and interactive/provider prerequisites.
- [x] 1.2 Add a command-tree fixture and identify stable IDs, category, scope and risk for each path.
- [x] 1.3 Confirm every current browser section links to the catalog and every unavailable item has actionable guidance.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add typed metadata and a completeness assertion against the runtime Clap command tree.
- [x] 2.2 Add the authenticated catalog endpoint and contract versioning.
- [x] 2.3 Add browser navigation, categories, search, availability/risk labels and CLI-only instructions.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check full top-level and nested command parity, duplicate IDs, route resolution and reasons for every non-web row.
- [x] 3.2 Check unauthorized, empty-search, disabled-provider, project-scope and narrow-screen behavior.
- [x] 3.3 Confirm catalog changes do not alter Clap invocation or existing command output.

## 4. Verification

- [x] 4.1 Run formatting, all-target compilation, focused catalog/API tests and strict OpenSpec validation.
- [x] 4.2 Run browser smoke and compare the rendered catalog to `forge --help` plus nested help output.
