# Design: Post-MVP readiness — README usage, terminal capture, vocabulary

## 1. Implementation boundary

**Repository:** `/home/paul/code/workspace-governance` — Python 3.11, standard
library only, plus Markdown documentation.

**Files to change:**

- `README.md` — add the Usage section, the terminal-capture embed, the
  screenshots-N/A note, and the Vocabulary section.
- `docs/assets/` (new) — `terminal-self-check.txt` (ANSI-stripped transcript)
  and an optional `terminal-self-check.cast`/`.svg`; redacted.
- `docs/vocabulary.md` (optional) — the definitions if the README section grows;
  links the canonical sources.
- `openspec/changes/post-mvp-readiness/` — this change package.

**Must not change:** `scripts/**` (including `workspace-check`),
`tests/**`, `schemas/**`, `profiles/**`, `templates/**`, `projects.json`,
`deployment/**`, `MATURITY_PROGRESS_SUMMARY.md`, `.project.json`, or any
canonical spec.

## 2. Language and runtime

Documentation is Markdown with fenced `bash` / `text` blocks. No new runtime is
introduced.

Exact commands:

```bash
python3 -m unittest discover -s tests
python3 scripts/workspace_check.py --self-check
python3 scripts/workspace_check.py --root .. --audit --json
node scripts/check-openspec-change-names.mjs
openspec validate --all --strict --no-interactive
git diff --check
```

## 3. Ownership and shared code

`workspace-governance` is the **shared owner** of the registry and evidence
vocabulary; this change documents that owner surface without changing it. The
consumers are the product repositories (via `.project.json`), `forge` (via the
read-only adapter), and `driftwatchdog` (the gate runtime). None of them owns a
second copy of the vocabulary.

- **Exported contract:** the documented command surface and the vocabulary
  definitions; the canonical sources remain `docs/capabilities.md`,
  `docs/product-code-quality.md`, `projects.json`, and the specs under
  `openspec/specs/`.
- **Consumers:** maintainers, product repositories, `forge`, and `driftwatchdog`.
- **Dependency direction:** products → governance.
- **Release boundary:** ships with this repository; no behavior or schema change.

## 4. Behavioral model

| Readiness gap | README / docs surface | Target state |
|---|---|---|
| Commands scattered across four sections | Usage section | One copy-paste example per command, each labelled with what it proves |
| No terminal evidence | Capture + screenshots-N/A note | A redacted capture shows `--self-check` (0/0) and `--audit --json` (healthy plus one finding); README states CLI-only |
| Vocabulary scattered | Vocabulary section / `docs/vocabulary.md` | One definition set that links the canonical sources and distinguishes intent from evidence |

**Vocabulary model (definitions link, never restate policy):**

| Term | Meaning | Canonical source |
|---|---|---|
| Declaration (`.project.json`) | Intent: what a project claims | `schemas/project.schema.json`, `openspec/specs/registry-integrity` |
| `evidence_status` | Verification state: `declared`, `configured`, `verified`, `unverified` | `docs/capabilities.md` |
| `capabilities` | Declared reusable platform capabilities with an evidence state | `docs/capabilities.md` |
| `release_evidence` | A producer-neutral record backing a deployment | `docs/capabilities.md` |
| `deployable: true` | Requires a promotion-ledger entry and complete `release_evidence` | `openspec/specs/deployable-batch-onboarding`, `docs/deployment-promotion.md` |
| Promotion ledger | Tamper-evident record in `deployment/promotions.json` | `docs/deployment-promotion.md` |
| Finding code | An audit result (ERROR/WARNING) with a stable name | `openspec/specs/registry-integrity` |

**Invariant:** a declaration is intent, not evidence; `deployable` requires both
a promotion entry and release evidence. The docs MUST NOT imply that registry
presence or a declaration is implementation evidence.

## 5. Contract and compatibility

- The README keeps every existing section and command; the change is additive.
- Every documented command is the shipped command; every capture link is
  repository-relative and resolves on disk; no external image host is introduced.
- The vocabulary section links the canonical sources and MUST NOT restate a
  policy in a way that could drift from them.
- No script, schema, template, registry, or spec behavior changes; the
  `workspace_check.py --self-check` result stays 0 errors / 0 warnings.

## 6. Failure and boundary policy

| Case | Result |
|---|---|
| Usage example names a flag or output the scripts do not produce | Correct to real behavior; readiness claim is FAIL |
| Capture contains a token, credential, host username, absolute private path, or secret value | MUST NOT be committed; regenerate redacted |
| Vocabulary restates policy instead of linking it | Correct to a link; a second source of truth will drift |
| README implies a declaration or registry entry is implementation evidence | Correct; that is exactly the confusion this change removes |
| Any `scripts/**`, `schemas/**`, `projects.json`, `deployment/**`, or canonical-spec file is touched | Out of boundary; revert |

## 7. Verification oracle

- `python3 -m unittest discover -s tests` — 122-test baseline stays green.
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings.
- `python3 scripts/workspace_check.py --root .. --audit --json` — same counts as
  before the change; the capture matches a real run.
- `node scripts/check-openspec-change-names.mjs` and
  `openspec validate --all --strict --no-interactive` — pass.
- Manual oracle: a consumer can open `README.md`, run a documented command, and
  resolve a vocabulary term to its canonical source.
- `git diff --check` — no whitespace errors.

## 8. Decision ledger

**Verified assumptions:**

- `python3 -m unittest discover -s tests` reports 122 tests, 0 failures; the
  audit is 0 errors / 18 warnings; `--self-check` is 0/0 (`HANDOFF.md`).
- The command surface is `workspace_check.py` (`--self-check` / `--audit` /
  `--project`), `auto_gate.py`, `product_code_quality.py`, `maturity_audit.py`,
  `rules_drift.py`, `init_project.py`, `promote_project.py`,
  `jenkins_manifest.py`, and `forge_governance_adapter.py`.
- The evidence vocabulary lives in `docs/capabilities.md`,
  `docs/product-code-quality.md`, `docs/deployment-promotion.md`, and the
  canonical specs; there is no top-level `LICENSE` file.
- The repository is CLI-only (no UI surface).

**Resolved alternatives:**

- (a) Screenshots vs terminal capture → chose **terminal capture**; there is no
  GUI, so screenshots are explicitly not applicable.
- (b) A `LICENSE` file vs docs-only → chose **docs-only**; the stated focus is
  README/docs, and adding a license is a separate metadata decision.
- (c) Restate the vocabulary vs link the canonical sources → chose **link**;
  `docs/capabilities.md` and the specs are the owners, and restating would drift.
- (d) Capture the full portfolio audit vs the governance self-check plus a
  synthetic finding → chose **the self-check plus an illustrative finding** so
  the capture is reproducible offline and contains no sibling-project data.

**Deferred:** a repository `LICENSE` file (separate metadata decision); the
`governance-front-door` skill (authored separately and links this README).

**Blockers:** none. If no asciinema renderer is available, ship the plain-text
transcript and record the tooling limitation; do not fabricate an SVG.

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| R1 Consolidated CLI usage | commands are discoverable | §4, §8(b) | `README.md` | example exists / invented flag | 1.1, 2.1 | every documented command runs `--help` |
| R2 Terminal capture, screenshots N/A | audit is visible | §4, §8(a) | `docs/assets/`, `README.md` | capture present / redaction failure | 1.2, 2.2 | capture matches a real run |
| R3 Vocabulary has one entry point | intent vs evidence is clear | §4, §8(c) | `README.md` / `docs/vocabulary.md` | term resolves / restated policy | 2.3 | each term links its canonical source |
| R4 Readiness stays docs-only | no behavior change | §1, §6 | diff scope | boundary review | 3.4 | self-check unchanged 0/0 |
| R5 Pre-existing dirty tree not swept in | only this change is staged | §1, §6 | change folder | stray file committed | 3.4 | diff scope check |
