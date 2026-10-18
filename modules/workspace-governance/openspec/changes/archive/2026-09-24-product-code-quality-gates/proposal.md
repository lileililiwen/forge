# Proposal: Enforce product-code boundaries and placeholder thresholds

## Why

The portfolio currently declares generic build, test, and security concerns,
but it has no shared, executable control that detects test or integration code
leaking into product source or placeholder-heavy product code. A project can
therefore report a structurally complete Gate while shipping code that is
unfinished or placed in the wrong boundary. AI-generated changes make this
failure mode common enough to require a deterministic repository check.

## What Changes

- Add a portable `scripts/product_code_quality.py` checker owned by Workspace
  Governance.
- Scan declared product/source roots while excluding test, fixture, example,
  generated, vendor, and build paths through explicit policy.
- Reject test-only product code; Rust test modules are permitted only when
  lexically contained by `#[cfg(test)]`, while Rust `tests/` remains test scope.
- Count placeholder markers in product code and fail when the configured
  threshold is exceeded. The default for newly adopted product roots is zero.
- Emit stable machine-readable findings and human-readable diagnostics with
  file, line, rule, and remediation fields.
- Provide a migration profile and fixtures for the projects already declaring
  `.ai-gate/gate.yaml`; projects without source-root metadata remain
  `REVIEW_REQUIRED`, not silently passing.

## BFS Impact Map

- **Capabilities:** new `product-code-quality-boundary` capability; existing
  registry, project declaration, and Gate declarations remain compatible.
- **Users and flows:** local project verification, Workspace Governance audit,
  and Driftwatchdog Gate consumers gain deterministic quality findings.
- **Contracts/data/persistence:** no registry schema or database change; the
  checker report uses a versioned JSON document and existing finding levels.
- **Integrations/configuration:** project metadata gains source roots,
  exclusions, language profile, and placeholder threshold; `.ai-gate` command
  bindings are added only where adoption is verified.
- **Callers:** `workspace_check.py`, `auto_gate.py`, project-local Gate
  commands, and future Forge/Driftwatch consumers.
- **Failure and boundaries:** missing roots/configuration is
  `REVIEW_REQUIRED`; test code in product roots is `FAIL`; threshold excess is
  `FAIL`; excluded test fixtures do not create findings; unreadable files are
  `REVIEW_REQUIRED`.
- **Tests:** language fixtures for Rust, Python, TypeScript, C#, and generic
  text; marker counting; exclusion and changed-scope behavior; JSON contract.
- **Dependencies:** Python standard library only; no AST or network service is
  introduced. Driftwatchdog consumes the command through its existing adapter.
- **Compatibility/privacy:** existing project declarations remain valid when
  the new policy is absent; reports contain paths and source snippets only
  within configured bounds and must redact configured secrets.

## Capabilities

- `product-code-quality-boundary`: Workspace Governance can identify test-code
  leakage and placeholder debt in declared product source.
- `portfolio-quality-gate-adoption`: adopted projects can bind the checker to
  their local Gate without moving product-specific rules into shared code.

## Non-goals

- No automatic code repair, test generation, or AI judgment.
- No claim that a passing lexical check proves behavior, integration, release,
  or runtime quality.
- No universal ban on tests in a repository; test roots remain first-class.
- No hard-coded assumption that every project uses Rust, Python, .NET, or
  JavaScript.
- No Forge implementation or replacement of Driftwatchdog's Gate executor.
