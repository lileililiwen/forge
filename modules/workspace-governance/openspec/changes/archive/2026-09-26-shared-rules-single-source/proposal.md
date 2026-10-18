# Proposal: Single-source the shared rules baseline

## Why

Every adopted project carries `.ai-rules/workflow.md` and `.ai-rules/completion.md`,
but there is no single source for what those files must say, and the portfolio
has drifted.

Two competing "canonical" copies already exist inside governance: the on-disk
`templates/project/.ai-rules/*` and a duplicate embedded `TEMPLATES` dict in
`scripts/init_project.py`. They disagree today — the embedded copy carries the
OpenSpec name-preflight sentence and the on-disk copy does not.

Measured on 2026-09-26 across the 75 projects that have `.ai-rules/workflow.md`:

- `workflow.md`: BFS model 74/75, OpenSpec name preflight 70/75, two-commit rule 57/75.
- `completion.md`: two-commit rule 56/75, "not runtime or release evidence" 46/75.
- 31 distinct file contents; sizes range from 499 to 3799 bytes, because many
  repositories legitimately extend the baseline with project-specific guidance
  (for example `ziban` writes in Chinese, `mewo` documents its own gates).

`crossalheart` has no `.ai-rules` at all, and that is the workspace audit's only
remaining 2 ERROR (`REQUIRED_FILE`).

Byte-forcing the template would destroy the project-specific guidance that makes
those files useful. An attempt to enforce *whole-paragraph* containment is
recorded in the design: it flagged 143 issues and appended a duplicate canonical
block to 60 files — including well-maintained ones such as `mewo` whose text
already expressed the same rules — and was reverted byte-for-byte. The invariant
is therefore **clause presence, reported as triage**, and the tool never
rewrites a project file.

## What Changes

- Make `templates/project/` the single canonical source and have
  `init_project.py` read it from disk; remove the embedded duplicate copy.
- Add `scripts/rules_drift.py --check`: a non-destructive triage report naming
  every adopted project that does not visibly carry an invariant clause of the
  baseline (`BFS`, the two-commit rule, the OpenSpec name preflight, the
  evidence boundary). It never rewrites a project file, and a project that
  documents the rules differently may opt out per file with
  `<!-- workspace-governance:rules-customized -->`.
- Detect a scaffold `HANDOFF.md` (the initializer boilerplate) so scaffolded
  docs cannot pass as real handoffs; surfaced by `workspace_check.py` as
  WARNING `HANDOFF_SCAFFOLD`.
- Keep the workspace audit sharp: clause drift stays a separate triage report
  rather than an audit failure, because repositories legitimately document the
  rules in their own words and language.
- Create `crossalheart`'s rules from the canonical template, clearing the
  audit's last two errors.

## Package Boundary and Split Assessment

**Single outcome:** one canonical rules baseline inside governance, with an
enforced, additive invariant across adopted projects.

**Included:** `templates/project/**`, `scripts/init_project.py`,
`scripts/rules_drift.py` (new), `scripts/workspace_check.py`, `tests/`, and
`crossalheart`'s missing rules.

**Excluded:** rewriting project-specific rule content; authoring real
`HANDOFF.md` bodies for product repositories; any driftwatchdog, Sisyphusfy,
platform or product source change; changing the two-commit process itself.

**Split signals considered:** the package pairs a generator/checker change with
a one-time additive sync of siblings. They share one owner
(`workspace-governance`), one contract (the canonical paragraph set), one state
model (contained / missing) and one oracle (`rules_sync --check` → 0). A
checker without the sync would leave the workspace failing; the sync without the
checker would drift again. They are the smallest independently verifiable unit.

**Dependencies:** package `registry-and-declaration-integrity` (shipped) — it
supplies the registry and the adopted-project list this check walks.

**Package map (see the archived `registry-and-declaration-integrity` proposal
for the full initiative map):** this is package 2 of 8. Packages
`portfolio-vocabulary-normalization` and `gate-run-evidence` depend on it;
`governance-front-door` depends on it.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| workspace-governance `templates/` | `templates/project/.ai-rules/workflow.md`, `completion.md` | the canonical paragraph text | stale: lacks the name-preflight sentence that 45 repositories already carry, and omits `README.md`/`ROADMAP.md` | workspace-governance; released with this repo | **extend shared owner** |
| `scripts/init_project.py` | embedded `TEMPLATES` dict | a second, disagreeing copy of the same text | duplicates the on-disk template | workspace-governance | **adopt** (read the on-disk template; delete the copy) |
| `scripts/product_code_quality.py` | `load_policy` / `evaluate` / `--json` | the established checker pattern (policy, exit codes, JSON report) | none; pattern reuse only | workspace-governance | **adopt** (mirror the checker shape) |
| `scripts/workspace_check.py` | `validate_project`, `profile_requirements` | the finding vocabulary and per-project hook | none | workspace-governance | **extend shared owner** |
| driftwatchdog | `src/gate/concerns.rs` | gate execution, not clause authoring | clause text is governance vocabulary, not a gate concern | driftwatchdog; unchanged | **not applicable** |
| sisyphusfy | `sisyphusfy/.project.json` | agent session supervision | no rules-baseline surface | sisyphusfy; unchanged | **not applicable** |
| product repositories | each `.ai-rules/*` | project-specific guidance worth preserving | must not be rewritten | each product owns its content | **keep local** (append only) |

No sibling implements a rules baseline, so no extraction is authorized; the
canonical owner is the existing shared owner, `workspace-governance`, and
project repositories keep ownership of their extra content.

## BFS Impact Map

- **Capabilities:** canonical templates, the initializer, the new rules checker.
- **Consumers:** `init_project.py`, `workspace_check.py`, every adopted product's
  `.ai-rules/*`, and any agent reading those rules.
- **Contracts:** the canonical paragraph set; `.ai-rules/<name>.md` containment;
  the sync marker comment appended to files it edits.
- **Integrations:** `auto_gate.py` (via the audit), the repository CI workflow.
- **Failure and boundary:** a missing paragraph is an ERROR; a scaffold
  `HANDOFF.md` is a WARNING; an unreadable file is a finding, never a pass;
  a repository without `.ai-rules` is created from the template on `--apply`.
- **Tests:** unit tests for paragraph extraction, missing-clause detection,
  idempotent apply, and scaffold-HANDOFF detection.
- **Dependencies:** the shipped registry (package 1) to enumerate adopted projects.
- **Compatibility, privacy, security:** additive only; existing project content
  is preserved byte-for-byte apart from appended paragraphs; rule files carry no
  secrets, and the audit's secret scan is unchanged.

## Capabilities

- `rules-baseline`.

## Non-goals

- Not rewriting or deleting any project's rule content; the drift tool is
  report-only and has no `--apply`.
- Not failing the workspace audit on wording drift; the audit surfaces only
  scaffold handoffs.
- Not authoring real `HANDOFF.md` bodies for product repositories.
- Not changing the two-commit process, the gate runtime, or any platform library.
