# Change workflow

## Selection and blocking preflight

Use the roadmap dependency order and `openspec list`; reconcile [HANDOFF](../HANDOFF.md) before implementation. First run the local blocking **check-openspec-change-names** preflight:

```sh
node scripts/check-openspec-change-names.mjs
```

Active change names must match `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$`. The checker ignores archive and hidden entries and rejects every invalid direct active directory. Failure blocks status/instructions selection, implementation, validation and archive until corrected. Run before `openspec status --change <name>` or `openspec instructions apply --change <name>`.

Set exactly one `current_spec: <active-change-name>` line at the first or last line of HANDOFF. Select only one eligible change; prerequisites require implementation evidence, not merely proposal completion.

## BFS → DFS → BFS

1. **BFS analysis:** map requirements, scenarios, concepts, modules, contracts, callers, persistence, integrations, tests, compatibility, concerns and verification before deep implementation.
2. **Structural pass:** update domain types, interfaces, DTOs, events, wiring, callers and test fixtures. Compilation is `SKELETON_READY`, not complete behavior.
3. **DFS implementation:** implement one coherent requirement/scenario through domain, application, infrastructure and integrations, with evidence for success, failure and boundaries.
4. **BFS verification:** revisit every original impact surface: requirements, callers, persistence, APIs, authorization, validation, logs, events, concurrency, compatibility, placeholders, required tests and applicable local Gate.

Every proposal uses Why, What Changes, BFS Impact Map, Capabilities and Non-goals. Designs resolve ownership, contracts, failures and migrations. Every tasks file uses fixed phases: BFS baseline; DFS requirement implementation; BFS regression/completeness; Verification. Unimplemented planning tasks stay unchecked.

## Local evidence and completion sequence

1. Update task checkboxes only when their outcomes are evidenced.
2. Run the change's actual formatting/build/test and integration checks locally. If tooling is absent, record the exact unavailable command and next action; never infer a pass.
3. Run name preflight, `openspec validate --all --strict --no-interactive`, relevant status/instructions checks and `git diff --check`.
4. Run the local Gate runtime before archive: rehearse with `forge gate --dry-run`, then run the full `forge gate` with a bounded timeout (for example `--timeout-secs 600`). "No shared Gate configured in CI" is never a reason to skip the local run. A Gate FAIL or unresolved REVIEW_REQUIRED attributable to the change blocks archive and completion. Pre-existing failures unrelated to the change do not block it, but must be recorded in HANDOFF with their remediation path, and the change must introduce no new failure. Record the verdict — or the exact blocking reason plus next action, never a pass — as a Gate row in the HANDOFF evidence table.
5. Review the original requirement and impact map, then archive the verified selected change without `--skip-specs`; canonical specs must be promoted.
6. Inspect the staged diff and commit only related implementation, tests, archive and promoted specs.
7. Update HANDOFF with evidence and the next active eligible change from `openspec list`. Advance the existing pointer in place; remove its line when no active changes remain. Never leave an archived ID, `none` or `TBD`.
8. Commit only HANDOFF, then stop. Do not start another change or push.

Planning-only documentation work stops after artifact verification: it does not archive or commit application work.

## Local concern routing

Read [quality and compatibility](concerns/quality.md) for all non-trivial changes; [security and privacy](concerns/security.md) for filesystem/process/credential/remote boundaries. Load [architecture](architecture.md) only for new modules, public abstractions, persistence or cross-cutting integrations. UI changes also follow the state/accessibility contracts of their selected UI or portal spec.

No generic Gate implementation, provider runner, installer or cache belongs in this repository merely to initialize docs.
