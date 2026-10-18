# Tasks: Forge Gate conformance for Workspace Governance

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Pin the module-local include and existing scanner boundary behavior; confirm global conventional-root behavior stays unchanged.
- [x] 1.2 Record the four scripts' public imports, command-line entry points, output contracts, and callers before extraction.
- [x] 1.3 Confirm Python 3.11 module ownership and the exact source-size/Gate verification paths; keep external siblings out of the write set.
- [x] 1.4 Confirm proposal, design, spec and task traceability align and the terminal UX behavior is actionable and accessible.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add a native module `gate.toml` preserving all enabled checks and explicitly including `scripts/**/*.py`; pin it with a manifest contract test.
- [x] 2.2 Split `workspace_check.py` into cohesive modules with the original CLI/import façade preserved; run its focused tests.
- [x] 2.3 Split `bind_gate_commands.py` into cohesive modules with the original CLI/import façade preserved; run its focused tests.
- [x] 2.4 Split `product_code_quality.py` into cohesive modules with the original CLI/import façade preserved; run its focused tests.
- [x] 2.5 Split `ui_conformance.py` into cohesive modules with the original CLI/import façade preserved; run its focused tests.
- [x] 2.6 Keep every maintained Python source file at or below 1,000 raw newline characters without exclusions or threshold changes.
- [x] 2.7 Ship `product_code_quality_claims.py` and `product_code_quality_scope.py` beside the quality-checker facade in Forge archives and installations; retain installed-binary behavior with a focused regression test.
- [x] 2.8 Document that the Forge-facing `gate.toml` takes precedence and that maintainers keep its selected commands aligned with `.ai-gate/gate.yaml`.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-run all existing scanner cases for explicit includes, excludes, symlink/binary handling, boundary uncertainty and deterministic findings; verify no global scanner change.
- [x] 3.2 Run the complete Workspace Governance Python suite, self-check, syntax validation and compatibility CLI smoke checks.
- [x] 3.3 Run Forge Gate against the module and verify nonzero candidate evaluation plus no source-size findings; inspect every other check result.
- [x] 3.4 Review all four extraction diffs for import cycles, lost symbols, changed output, omitted code and files still over the limit.
- [x] 3.5 Confirm no external sibling files, schema/wire formats, or Forge UI changed; include only related Forge packaging and installed-resource tests for the split helpers.

## 4. Verification

- [x] 4.1 Run relevant existing scanner tests and ensure Forge's Gate binary reflects the unchanged scanner implementation.
- [x] 4.2 Run Forge Gate dry-run and full Gate on `modules/workspace-governance`; record actual result and revision in HANDOFF.
- [x] 4.3 Run Python suite, governance self-check, script syntax and CLI checks, installed-resource/distribution tests, shell syntax, scanner tests, OpenSpec name preflight, strict validation, and `git diff --check`.
- [x] 4.4 Archive with canonical spec promotion, commit only related implementation/tests/spec/archive, then make the HANDOFF-only evidence/pointer commit.
