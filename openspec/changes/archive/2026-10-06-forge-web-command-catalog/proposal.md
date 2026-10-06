# Proposal: Make CLI coverage visible in the web command center

## Why

Forge has dozens of top-level commands and nested workflows, while the new browser page presents only a project roster. Users cannot tell what Forge can do, where a workflow lives, or whether an absent action is unsupported, CLI-only or temporarily unavailable.

## What Changes

- Define an exhaustive, versioned browser command catalog derived from the actual Rust Clap command tree.
- Give each command/subcommand a stable ID, user-facing label, category, owning project scope, capability state, risk class, browser route or explicit CLI-only reason, and evidence source.
- Organize web workflows by task and project context; provide search and clear unavailable/permission/provider states.
- Add a completeness check that fails when a Rust CLI command is missing from the catalog or mapped without a valid web route/reason.

## Package Boundary and Split Assessment

This is the second package in the queue after `forge-web-project-fleet`. Its single outcome is discoverability and complete coverage accounting; it does not implement the underlying workflows. Project actions, portfolio controls and remote delivery have separate persistence, risk and test oracles and remain three consumer packages.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge CLI command tree | `src/main.rs::Commands` and nested `*Commands` enums | Authoritative executable command definitions and help text | Clap metadata does not include UI labels, risk, availability or route | Forge CLI/API/web ship together | **extend shared owner** |
| Forge portal | `openspec/specs/control-plane-portal/spec.md` | Twelve named portal sections and state/accessibility requirements | Does not account for all current CLI groups | Forge portal contract | **extend shared owner** |
| AllTools | User-provided Rust template path | Visual sign-in reference | No matching command/API contract | Independent project | **keep local** |

## BFS Impact Map

- **Users:** signed-in Forge operators discover commands by outcome and project.
- **Contracts:** versioned catalog, stable command IDs, per-command availability and reason.
- **Callers:** standalone frontend, API, and Rust Clap command definitions.
- **Completeness:** catalog checks enumerate top-level and nested commands; updates to Clap commands require matching browser disposition.
- **Security:** catalog is descriptive; authorization is still enforced by each invoked JSON operation. Risk labels do not grant permission.
- **Verification:** command-tree parity test, catalog schema/API tests, browser search/category/CLI-only state tests.
- **Unaffected:** operation implementations, registry schema, project authentication and provider execution.

## Capabilities

- `forge-web-command-catalog`: account for the complete CLI command tree in the authenticated web experience.

Source requirements: `requirement.md` §36; canonical `control-plane-portal` and actual `src/main.rs` Clap tree.

## Non-goals

- This catalog alone does not make an action executable in the browser.
- No shell command interpolation from user input and no hidden automatic invocation of CLI binaries.
- No false “supported” state for missing APIs, disabled providers, or unavailable project capabilities.
