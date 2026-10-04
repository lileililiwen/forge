---
name: forge-project-workflow
description: Use when creating, refining, inspecting, checking, releasing, or publishing a project through Forge, or when a Forge task crosses into workspace governance, shared kits, GitHub, Gate, OpenPanel, Hypora, or Hermora.
---

# Forge project workflow

Forge owns the human-facing project factory and coordination surface. Read the
current `README.md`, `HANDOFF.md`, `ROADMAP.md`, `docs/architecture.md`, and
the relevant canonical OpenSpec spec before selecting an operation. Reconcile
`openspec list` with `HANDOFF.md`; a package or archived spec alone does not
prove a live provider result.

## Route by operation

| Need | Start here | Owner at the boundary |
| --- | --- | --- |
| Create, import, inspect, or upgrade | `README.md` command map and profile/kit specs | Forge; generated project owns source and native build/test |
| Shared kit or capability | kit descriptor, pinned version, ownership receipt, consumer manifest | Language platform and platform-contracts own packages and wire shapes |
| Governance or maturity | local doctor plus optional versioned provider observation | Workspace Governance owns workspace declarations and audit |
| Quality evidence | `forge gate` and Gate evidence spec | Driftwatchdog owns execution, policy, run history, and export |
| Repository distribution | GitHub workflow spec and authenticated `gh` session | GitHub owns remote state; local evidence remains authoritative |
| Publish and operate | project-to-production workflow and publish specs | OpenPanel deploys and promotes; Hermora enrolls after publish |
| Validated idea import | Hypora import spec and platform idea-graduation contract | Hypora exports approved aggregate evidence; Forge previews before creation |

## Decision rules

1. Inspect the selected project's profile, manifest, native toolchain, local
   changes, and the operation's exact preconditions. Treat deterministic assets
   and pinned plans as the baseline; generated projects must work without a
   Forge runtime or sibling checkout.
2. Use versioned contracts at cross-project seams. Keep governance, Gate,
   deployment, and published-site operations with their owners. Never create a
   source-level sibling dependency or copy a shared contract shape silently.
3. Preserve the separation between plan, scaffold, native build, runtime
   verification, provider success, and deployability. Record failures and
   unavailable or stale evidence without promotion to healthy or PASS.
4. For remote writes, publishing, deployment promotion, and Hermora enrollment,
   use the explicit stage and authority in the owning workflow. Reuse the
   developer's authenticated `gh` session; do not extract tokens or add a
   second login. Retry Hermora enrollment separately from deployment.
5. For Forge source changes, follow `AGENTS.md`: one eligible OpenSpec change,
   BFS to DFS to BFS, local verification, archive, implementation commit,
   HANDOFF-only commit, then stop without pushing.

Use the owner repository's current files for exact commands and status. Forge's
roadmap records delivered contracts, while real product, Gate, GitHub, browser,
and provider outcomes require their own current evidence.
