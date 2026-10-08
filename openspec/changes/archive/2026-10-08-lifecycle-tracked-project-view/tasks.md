# Tasks: Lifecycle-tracked project view

## 1. BFS — Baseline and impact coverage

- [x] Read every file involved: `frontend/index.html`
  (workbench cards, "Command catalog (reference)" section,
  nav), `frontend/app.js` (`renderCommands`, `loadCommands`,
  `showCommandsError`, `renderWorkflows`, `renderProjectActions`),
  `frontend/styles.css` (`.command-*`, `.depth-*`,
  `.workflow-list`), the `forge-web-command-catalog` /
  `-execution` / `-workflows` specs, the
  `forge_web_command_catalog_contract` test file.
- [x] Map every consumer of the catalog JSON to its site
  (the workbench is the only consumer after this change).
- [x] Add the OpenSpec change directory with proposal,
  design and a `lifecycle-tracked-project-view` delta spec.
- [x] Confirm the package is implementable by one bounded
  pass without inventing architecture or an owner.

## 2. DFS — File-by-file change

- [ ] `frontend/index.html`: remove the
  `<section id="commands">` block; remove the "Commands"
  nav link; reorder the workbench cards so the new
  "Lifecycle" card is first, the "Manage this project"
  card (renamed from "Project actions") is second, the
  upgrade card is third, and the read-only cards
  (manifest, health, status, delivery, evidence) are last;
  add the lifecycle detail list with `wb-lifecycle-*` ids
  for maturity, target, profile, path, and last update.
- [ ] `frontend/app.js`: drop `renderCommands`,
  `showCommandsError`, `renderWorkflows`, the
  catalog-page's filter event handlers, the
  catalog-page's `command-search` / `category-filter` /
  `availability-filter` / `command-count` /
  `command-no-results` / `commands-table` lookups, and
  the catalog's `command-rows` rendering; add
  `renderLifecycle(manifest)` and `describeMaturity(level)`;
  add `LIFECYCLE_STAGES` and `lifecycleStageFor(command)`
  in `renderProjectActions`, which now groups the
  executable rows by stage instead of rendering one flat
  list; `loadCommands` keeps the JSON fetch but no longer
  populates the catalog-page filter widgets.
- [ ] `frontend/styles.css`: drop the catalog-page styles
  (`.command-table`, `.command-id`, `.command-path`,
  `.command-guidance`, `.depth-1`, `.depth-2`); add
  `.wb-lifecycle`, `.lifecycle-list`, `.wb-actions-grouped`,
  `.wb-actions-group`, `.wb-actions-head`,
  `.wb-actions-help` for the new cards and grouped buttons.
- [ ] `tests/forge_web_command_catalog_contract.rs`: replace
  `frontend_ships_the_catalog_as_labels_never_execution`
  with
  `frontend_uses_the_catalog_for_per_project_buttons_not_a_reference_page`,
  which asserts the catalog page is gone and the workbench
  groups the catalog rows as buttons.

## 3. BFS — Cross-surface regression and completeness

- [ ] `forge_web_command_workflows_contract` (5 tests) and
  `forge_web_command_execution_contract` (5 tests) stay
  green.
- [ ] `forge_admin_api_contract` (4 tests) stays green.
- [ ] `cargo test --bin forge` (catalog parity) stays
  green.
- [ ] The catalog JSON endpoint `/v1/admin/commands` returns
  the same shape.
- [ ] The CLI surface and the registry/journal schema are
  unchanged.
- [ ] The `forge-web-command-catalog` /
  `forge-web-command-execution` /
  `forge-web-command-workflows` spec files are still
  present and unchanged.

## 4. Verification

- [ ] `cargo fmt` then `cargo fmt --check` clean.
- [ ] `cargo build` 0 errors.
- [ ] `cargo test --workspace` — every surviving test
  green; the renamed test in the catalog contract is
  green.
- [ ] `node scripts/check-openspec-change-names.mjs` PASS
  and `openspec validate --all --strict --no-interactive`
  0 failures with this change active.
- [ ] `git diff --check` clean; review newly added files.
- [ ] `forge gate` local run with verdict recorded; this
  change is a strict refactor of the browser surface, so
  no new gate finding is expected.
- [ ] Manual smoke: `scripts/web.sh start
  --projects-root <path>` → open the dashboard → open a
  project from the fleet → the workbench shows the
  lifecycle card, then the actions card grouped by stage,
  then the upgrade workflow, then the read-only cards;
  the "Command catalog (reference)" page is no longer
  in the nav.
- [ ] Archive with `openspec archive
  lifecycle-tracked-project-view --yes` (no
  `--skip-specs`); promote the canonical spec; update
  HANDOFF with the evidence and pointer handling.
