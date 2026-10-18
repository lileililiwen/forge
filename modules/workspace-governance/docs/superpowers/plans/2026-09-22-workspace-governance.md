# Workspace Governance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a central registry, mandatory checker, bootstrap template, and Jenkins deployment manifest for the local project portfolio.

**Architecture:** A standard-library Python checker reads JSON registry and project declarations. The governance repository owns schemas and policy; consumer repositories retain only declarations and project-specific exceptions. Jenkins consumes a generated manifest and remains the execution layer.

**Tech Stack:** Python 3.11+ standard library, JSON, YAML documentation, Bash, Node.js OpenSpec preflight, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-09-22-workspace-governance-design.md`

## Global Constraints

- Missing mandatory checks are failures.
- Secrets never appear in registry or generated deployment output.
- The checker is deterministic and works without third-party Python packages.
- Deployment requires an explicit `deployable: true` declaration and a passing governance check.
- Existing repositories are audited before migration; no product source is rewritten by bootstrap tooling.

### Task 1: Create the governance repository contract

**Files:**
- Create: `README.md`, `ROADMAP.md`, `HANDOFF.md`, `AGENTS.md`
- Create: `.ai-rules/workflow.md`, `.ai-rules/completion.md`
- Create: `.project.json`, `.gitignore`, `.agentignore`

- [ ] Write the repository documents with the ownership model, adoption steps, verification evidence, and two-commit OpenSpec rule.
- [ ] Run link and required-file checks.

### Task 2: Add registry, schemas, and profiles

**Files:**
- Create: `projects.json`
- Create: `schemas/project.schema.json`, `schemas/registry.schema.json`, `schemas/deployment.schema.json`
- Create: `profiles/*.json`
- Create: `deployment/projects.json`

- [ ] Register discovered repositories with explicit exceptions for shared infrastructure, learning, scratch, and deployment-only directories.
- [ ] Define project, registry, and deployment schemas.
- [ ] Ensure deployment output contains no credentials or secret values.

### Task 3: Implement mandatory validation

**Files:**
- Create: `scripts/workspace_check.py`
- Create: `scripts/workspace-check`
- Create: `scripts/check-openspec-change-names.mjs`
- Create: `tests/test_workspace_check.py`

- [ ] Implement registry validation, project declaration validation, governance-file checks, OpenSpec checks, and deployment checks.
- [ ] Emit human-readable findings and JSON output.
- [ ] Add tests for pass, missing declaration, invalid OpenSpec name, missing CI, and secret rejection.

### Task 4: Adapt project initialization

**Files:**
- Create: `scripts/init_project.py`
- Create: `templates/project/.project.json`
- Create: `templates/project/.ai-gate/gate.yaml`
- Create: `templates/project/AGENTS.md`, `templates/project/HANDOFF.md`
- Create: `templates/project/.ai-rules/workflow.md`, `templates/project/.ai-rules/completion.md`

- [ ] Implement a non-destructive initializer for existing or new repositories.
- [ ] Support `--check-only`, `--force` only for missing files, and explicit project ID/profile arguments.
- [ ] Keep shared runtimes out of generated repositories.

### Task 5: Add Jenkins integration

**Files:**
- Create: `scripts/jenkins_manifest.py`
- Create: `.github/workflows/governance.yml`
- Modify: `README.md`, `deployment/projects.json`

- [ ] Generate an eligible-project manifest for Jenkins.
- [ ] Require a successful governance check before marking a project deployable.
- [ ] Document the Jenkins-local installation and deployment sequence without embedding credentials.

### Task 6: Verify and hand off

- [ ] Run `python3 -m unittest discover -s tests -v`.
- [ ] Run `python3 scripts/workspace_check.py --root .. --registry projects.json --json` from a checkout where sibling projects are present.
- [ ] Run `python3 scripts/jenkins_manifest.py --check`.
- [ ] Run `git diff --check` and inspect the final status.
- [ ] Record exact evidence and known migration gaps in `HANDOFF.md`.
