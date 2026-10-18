---
name: workspace-governance
description: Use when a portfolio task touches Workspace Governance, Forge, another manager project, shared platform contracts, project declarations, quality evidence, release readiness, or cross-project ownership and cooperation.
---

# Workspace Governance

One front door, not a new authority. When the task touches the workspace
registry, a project declaration, a gate, evidence, shared rules, platform
adoption, or shared contracts, start here, follow the routing table to the
named reference, and run the reference's commands. Never invent a mechanism
beside an existing one.

## Routing table

| Situation | Route |
|---|---|
| Is the workspace healthy / why is the audit red | `skills/workspace-governance/references/declaration.md`, then `python3 scripts/workspace_check.py --root .. --audit` |
| Add or fix a project declaration | `skills/workspace-governance/references/declaration.md` (`scripts/init_project.py`, `schemas/project.schema.json`) |
| Gate, evidence, or release readiness | `skills/workspace-governance/references/gate-and-evidence.md` (driftwatchdog, `docs/capabilities.md`, `scripts/promote_project.py`) |
| Shared capability adoption finding, or permission to adopt one | `skills/workspace-governance/references/platform-adoption.md` (`capabilities.json`, `docs/capabilities.md`, `docs/adoption-grant.md`) |
| Kind, profile, marker, or secret vocabulary | `skills/workspace-governance/references/vocabulary.md` (`vocabulary.json`) |
| Shared rules, `.ai-rules`, or handoff shape | `skills/workspace-governance/references/rules-baseline.md` (`scripts/rules_drift.py`, `openspec/specs/rules-baseline/spec.md`) |
| License, discoverability, GitHub metadata, or repository publication | `skills/workspace-governance/references/repository-metadata.md` (`scripts/repository_metadata.py`, `scripts/github_metadata.py`, `scripts/init_project.py`) |
| Shared library adoption (.NET / Rust) | `skills/workspace-governance/references/platform-adoption.md` (dotnet-platform-libs, rust-platform-libs, `docs/capabilities.md`) |
| Writing or changing any UI, auth page, theme, or design token | `skills/workspace-governance/references/ui-convention.md` (`python3 scripts/ui_conformance.py <this-repo>`, `python3 scripts/bind_ui_gate.py <this-repo>`); for adopting a repo to the kit or draining the baseline, the `ui-kit-adoption` skill (`skills/ui-kit-adoption/SKILL.md`) |
| Shared contracts or schemas | `skills/workspace-governance/references/contracts.md` (platform-contracts) |
| Forge and common-project cooperation, project factory, idea graduation, publishing, or post-publish operations | `skills/workspace-governance/references/portfolio-cooperation.md` (owner and contract routing) |
| Unknown or ambiguous | Safe default: `python3 scripts/workspace_check.py --root .. --audit`, then `openspec list`. Say that no route matched; never guess. |

## Boundaries

- **Work directly on `main` by default; do not create a branch or worktree
  unless the user asks for one or designates one.** This is canonical shared
  baseline text, not a preference of this repository. It is carried in
  `templates/project/.ai-rules/workflow.md` and `templates/project/AGENTS.md`,
  copied to new projects by `scripts/init_project.py`, and reported
  fleet-wide as the `work-on-main rule` clause of
  `python3 scripts/rules_drift.py --root .. --check`. A repository that
  documents it in its own words opts out per file with
  `<!-- workspace-governance:rules-customized -->`. The report is triage, not a
  gate — see `skills/workspace-governance/references/rules-baseline.md` for
  that limit and the `rules-drift-gate-wiring` follow-up.
- Read first: `python3 scripts/workspace_check.py --self-check` passes on a
  healthy checkout. The audit is authoritative; `scripts/maturity_audit.py`
  only reports its counts.
- Destructive and shared-state actions — pushing, deploying, promoting, or
  re-promoting — stay behind the existing explicit-approval boundary. This
  skill never instructs them; the references link the owning scripts and docs
  so an approved operator can run them.
- References link canonical scripts and specs; they never restate policy.
  `python3 scripts/check_skill_refs.py` proves every cited path exists.

## Install

This skill is versioned with the scripts it cites and is not installed
automatically. See `README.md` for the documented symlink-or-copy install
into the agent skills directory.
