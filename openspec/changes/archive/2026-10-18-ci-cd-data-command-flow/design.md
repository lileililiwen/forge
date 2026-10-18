# Design: CI/CD system model documentation

## Implementation boundary

Edit only the new `docs/ci-cd-system-model.md`, README navigation and the stale Gate API sentence; author this OpenSpec change and record its completion in `HANDOFF.md`. Runtime code, schemas, templates, CI workflows, providers, and portal behavior are out of scope.

## Language and runtime

Markdown documentation in the Forge repository. No new executable, package, dependency, or runtime. Validate links and source references by repository inspection; run `git diff --check`, OpenSpec name preflight and strict validation, and the required local Forge Gate.

## Ownership and shared code

Forge owns the guide because it describes Forge's composition and boundaries. Do not duplicate an external contract: link and explain the existing Forge Gate, standard-pack, release, and delivery contracts. GitHub and configured providers retain their external execution ownership.

## User experience and interface

Reader journey: README link → scope and architecture picture → choose local hook, generated CI, Forge CI/release, release workflow, or delivery workflow → inspect evidence/gaps and source links. Organize by flow and state transitions; use Mermaid diagrams and tables for relationships. Keep diagrams readable at narrow widths through modest node counts and provide textual explanations. Standard Markdown headings, link labels, and tables provide keyboard/screen-reader structure. No browser route or interactive states change.

## Behavioral model

The guide represents current behavior as separate flows. Gate execution writes project-local revision-bound JSON and journals a verdict in the local Forge registry. Release preflight reads Gate evidence and captures release checks; release apply executes its configured stages. Delivery preflight invokes its configured provider and journals an operation; stage requires same-project/revision preflight confirmation; promotion requires a healthy same-revision stage and explicit revision confirmation. Generated CI runs on GitHub-hosted runners; the current templates do not demonstrate transfer of their Gate evidence into the developer's registry or Forge workbench. The guide must explicitly label that last conclusion as source-based inference.

## Contract and compatibility

Use the current README/template pins and canonical spec versions as inspected at author time. Link source files with repository-relative paths so the guide remains portable across checkouts. Correct README to state that Gate is absent from MCP but status and explicit re-run are available through authenticated admin HTTP routes.

## Failure and boundary policy

Documentation must distinguish PASS, fail/blocked, stale, absent/unverified, unavailable, and provider-specific terminal outcomes. Do not infer successful production deployment from a successful CI workflow or Gate result. Mark branch protection and live GitHub/provider configuration as not verified by source inspection. If any reference cannot be confirmed, omit the claim or label it unresolved.

## Verification oracle

- Every asserted command/record path maps to current source, a current template, or a canonical spec.
- Mermaid diagrams agree with prose and evidence ownership tables.
- All internal Markdown links resolve.
- `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, `git diff --check`, Gate dry-run and full Gate are recorded truthfully.
- The guide does not claim the missing flow is implemented.

## Decision ledger

- **Resolved:** document current behavior and prioritized gaps in one guide; do not implement a new pipeline contract in this task.
- **Resolved:** keep GitHub job status/artifact ownership with GitHub and Gate policy/evidence production with Forge.
- **Deferred:** CI evidence ingestion, release-to-delivery composition, and service topology/local runtime require independent design and verification packages.
- **No blocker:** source and current canonical specifications provide enough evidence for a current-state model.
