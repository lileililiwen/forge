# Repository Discoverability Governance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect projects that lack standard license/discoverability metadata and provide safe Python and GitHub CLI workflows to repair and verify them.

**Architecture:** `workspace_check.py` remains read-only and reports missing or invalid local metadata. `init_project.py` gains additive local repair for `LICENSE` and `.project.json.discoverability`. A separate `github_metadata.py` owns authenticated GitHub inspection and explicit metadata publication, with subprocess boundaries injectable in tests. GitHub publication evidence is kept separate from local declarations.

**Tech Stack:** Python 3.11+ standard library, JSON Schema, `unittest`, GitHub CLI (`gh`), Git.

**Spec:** `openspec/changes/repository-discoverability-governance/`

## Global Constraints

- Audit mode is read-only and must not mutate project files, Git remotes, or GitHub repositories.
- Local repair is additive by default and must preserve existing declarations, licenses, README content, remotes, and unrelated worktree changes.
- GitHub operations require successful `gh auth status`; authentication failure is blocking and instructs manual `gh auth login`.
- No token, secret, or machine-specific absolute path may be written to project metadata or evidence.
- Publication declarations are not publication evidence; remote and GitHub metadata must be independently verified.

### Task 1: Add failing tests for metadata validation and repair

**Files:**
- Modify: `tests/test_workspace_check.py`
- Modify: `tests/test_init_project.py`
- Create: `tests/test_repository_metadata.py`
- Create: `tests/test_github_metadata.py`

- [ ] Add tests for missing, incomplete, malformed, and valid discoverability blocks.
- [ ] Add tests proving audit findings are read-only and distinguish missing license from metadata drift.
- [ ] Add tests for additive local repair and preservation of existing metadata/license.
- [ ] Add tests that invalid `gh auth status` blocks GitHub operations and that valid auth builds `gh repo edit` arguments without secrets.
- [ ] Run the focused tests and observe the expected failures before implementing production code.

### Task 2: Implement the local metadata contract

**Files:**
- Create: `scripts/repository_metadata.py`
- Modify: `schemas/project.schema.json`
- Modify: `scripts/workspace_check.py`
- Modify: `profiles/*.json`

- [ ] Define the required `discoverability` fields and `publication` evidence shape.
- [ ] Add strict validation for field types, non-empty descriptions, normalized topics, SPDX license identifiers, and visibility values.
- [ ] Add profile-driven required `LICENSE` and discoverability checks while preserving the inventory profile exception.
- [ ] Emit stable findings: `LICENSE_MISSING`, `DISCOVERABILITY_MISSING`, `DISCOVERABILITY_INCOMPLETE`, `DISCOVERABILITY_INVALID`, and `PUBLICATION_UNVERIFIED`.
- [ ] Keep audit behavior read-only and compatible with JSON output and existing exit codes.

### Task 3: Implement additive local repair

**Files:**
- Modify: `scripts/init_project.py`
- Modify: `templates/project/.project.json`
- Create: `templates/project/LICENSE`
- Modify: `README.md`
- Modify: `tests/test_init_project.py`

- [ ] Add explicit metadata arguments and an apply path that writes only missing local fields/files.
- [ ] Add MIT license generation only when no license exists and the operator explicitly selects the default or passes `--license-spdx MIT`.
- [ ] Refuse to invent semantic descriptions, audiences, use cases, or search phrases when required values are absent.
- [ ] Preserve existing `.project.json`, README, LICENSE, and unrelated files byte-for-byte.
- [ ] Run focused tests and then the full unit suite.

### Task 4: Implement authenticated GitHub metadata verification/publication

**Files:**
- Create: `scripts/github_metadata.py`
- Create: `tests/test_github_metadata.py`
- Modify: `README.md`
- Modify: `skills/workspace-governance/SKILL.md`
- Modify: `skills/workspace-governance/references/declaration.md`

- [ ] Add read-only `--check` for auth, origin identity, remote existence, GitHub description/homepage/topics, and default branch evidence.
- [ ] Add explicit `--apply` for `gh repo edit`; preserve existing topics and use additive `--add-topic` operations.
- [ ] Add explicit `--publish` for `gh repo create --source=. --remote=origin --push` only when identity and visibility are unambiguous.
- [ ] Block invalid/missing auth and report the manual `gh auth login` command without handling credentials.
- [ ] Record machine-readable publication state and evidence references without embedding tokens.

### Task 5: Cross-surface verification and OpenSpec handoff

- [ ] Run focused tests, full unit tests, self-check, workspace audit, skill-reference check, and strict OpenSpec validation.
- [ ] Run CLI `--help` checks for the new scripts.
- [ ] Confirm unrelated dirty files remain unchanged and inspect the staged diff.
- [ ] Update `HANDOFF.md` with the active pointer, implementation evidence, and remaining migration findings.
- [ ] Archive the change and create exactly two commits: implementation/tests/archive/generated specs, then `HANDOFF.md` only.
