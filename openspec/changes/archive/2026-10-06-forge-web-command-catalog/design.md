# Design: forge-web-command-catalog

## Implementation boundary

Forge Rust 2021 command metadata and authenticated API read endpoint, consumed by standalone `frontend/`. Inspect `src/main.rs` Clap enums, API router and frontend navigation. Do not move HTML into Rust or expose a generic arbitrary-command endpoint.

## Language and runtime

Rust 2021 / existing API and browser-native HTML/CSS/JS. No new dependencies. Verify via `cargo fmt --check`, `cargo check --all-targets`, focused catalog/API tests, web browser tests and strict OpenSpec validation.

## Ownership and shared code

The Rust CLI owns the command-tree source of truth; a typed static metadata catalog is co-located with CLI/API definitions. The API serializes metadata only. Frontend owns grouping, search and accessible presentation. No shared sibling package is appropriate for Forge-specific commands.

## Behavioral model

Catalog rows use `{id,parent_id,label,summary,category,scope,risk,availability,route,cli_invocation,reason,capabilities}`. Availability is one of `web`, `cli_only`, `provider_required`, `project_capability_required`, `disabled`, or `not_yet_web`; every non-web state carries a plain-language reason and next step. Risk is `read`, `local_write`, `remote_write`, or `session_admin`. IDs follow Clap paths (for example `project.github.observe`). All top-level groups are represented: list, inspect, register, import, graduation, profile, kit, new, doctor, check, feature, upgrade, spec, agent, test, commit, push, mcp, mirror, docs, release, deploy, publish, component, ui-pattern, intent, procedure, identity, analytics, api, web, portal, portfolio, readiness, provider, governance, fleet, project, gate, contract, inventory, standard, remediate, describe, classify, delivery, studio, help. Every nested command is also individually represented. The current tree includes nested groups for delivery, studio, profile, kit, feature, spec, MCP, docs, release, deploy, publish/provider, component, UI-pattern, intent, procedure, identity, analytics, API, web, portal, portfolio and all its subgroups, graduation, readiness, provider, governance, fleet, project/GitHub, contract, inventory, standard, remediate, describe, classify and agent.

Coverage checks compare the runtime Clap tree against catalog IDs. Web rows must resolve to an implemented typed JSON route. CLI-only rows name why browser execution is unsafe, interactive, transport-level, build-time or otherwise unsupported and link to the command help. Catalog `not_yet_web` is allowed during staged delivery but remains a visible tracked gap and cannot be labeled supported.

## Contract and compatibility

`GET /v1/admin/commands` requires the Forge global session and returns `{commands, categories, contract}`. API exposes no shell/eval route. Stable IDs are additive; removing/renaming an ID requires a contract migration. Existing CLI usage and outputs remain unchanged.

## Failure and boundary policy

Unauthenticated requests return 401; malformed static metadata prevents build/test; unknown category/search returns an empty result state; a route whose API capability is absent is unavailable and cannot render an enabled action. Catalog read failures return 503. User-provided strings are search terms only and never become shell arguments.

## Verification oracle

Enumerate every top-level and nested Clap command in a test and assert exactly one catalog row per path, no duplicate IDs, valid parent references, valid state/reason rules, and resolvable route IDs. Browser tests verify all categories, search, risk/state labels and CLI-only guidance. Every named top-level command from `forge --help` is asserted, including help and transport commands.

## Decision ledger

- Resolved: web availability is evidence-based; catalog is not a promise that a backend exists.
- Resolved: the catalog describes CLI commands and nested actions, while workflow packages implement supported web actions.
- Resolved: transport/developer commands remain discoverable with CLI guidance where browser semantics are wrong.
- Deferred: per-user role-based filtering; Forge currently has a single global administrator.
- No blockers.

## Requirement traceability

| Requirement | Design decision / boundary | Success, failure and boundary scenarios | Task IDs | Verification oracle |
|---|---|---|---|---|
| Exhaustive CLI command coverage | Typed metadata alongside Rust Clap tree | added command fails completeness; complete catalog | 2.1, 3.1 | enumerate actual Clap paths and assert one row per path |
| Truthful availability and safe navigation | Route/state/reason tuple; no shell endpoint | provider unavailable; terminal-only; empty search | 2.2–2.3, 3.2 | valid route resolution, actionable non-web states, literal search |
| Catalog authorization and invocation isolation | global-session read endpoint only | anonymous denied; shell syntax remains text | 2.2, 3.2 | 401 test and assert no process invocation path exists |
