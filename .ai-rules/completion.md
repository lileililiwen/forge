# Completion conditions

Work is DONE only when all applicable stopping conditions hold:

- Every scoped requirement and success/failure/boundary scenario has current evidence.
- All impacted callers, contracts, persistence, adapters and compatibility paths have migrated.
- No current-change placeholder, disabled behavior, fabricated result or unresolved scope gap remains.
- Required local formatting, builds, tests and integration checks pass; generated profile support has native-tool evidence. Missing tools and unavailable providers are blockers to their verification claims.
- The blocking **check-openspec-change-names** command `node scripts/check-openspec-change-names.mjs` passes before status/instructions selection and before strict validation/archive.
- `openspec validate --all --strict --no-interactive` and diff/related-file review pass before archive.
- Applicable local Gate checks have run before archive. No Gate is configured today; if configured later, FAIL or unresolved REVIEW_REQUIRED prevents completion.
- Original BFS impact surfaces have received final regression and completeness review.
- Task checkboxes match the evidence; incomplete or blocked outcomes are reported with exact failed command and next action.
- Archive promotes canonical specs, related changes are committed, and HANDOFF points to the next active change or omits the pointer when none remain.

Build success, test success, strict artifact validation and `SKELETON_READY` are not individually DONE. CI is a second verification layer, never the first check. Planning-only packages may be declared structurally validated but never implemented.
