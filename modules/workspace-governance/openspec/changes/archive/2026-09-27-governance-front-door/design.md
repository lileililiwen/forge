# Design: One front-door skill that routes to the governance references

## 1. Implementation boundary

**Repository:** `/home/paul/code/workspace-governance` — Markdown plus a small
stdlib Python check.

**Files to change:**

- `skills/workspace-governance/SKILL.md` (new) — front matter with one broad
  trigger description, then a routing table.
- `skills/workspace-governance/references/{declaration,gate-and-evidence,vocabulary,rules-baseline,platform-adoption,contracts}.md` (new).
- `scripts/check_skill_refs.py` (new) — extract every backticked path and
  `python3 scripts/*.py` command from the skill and its references and fail when
  one is missing.
- `tests/test_skill_refs.py` (new).
- `README.md` — an install section.

**Must not change:** any checker, any canonical spec, `templates/`, or any
sibling repository.

## 2. Language and runtime

Markdown for the skill and references; Python **3.11** standard library for the
reference check. Commands:

- `python3 scripts/check_skill_refs.py`
- `python3 -m unittest discover -s tests -v`
- `openspec validate --all --strict --no-interactive`

## 3. Ownership and shared code

Project-local to `workspace-governance`. No extraction.

- **Exported contract:** the skill's trigger description and the reference file
  names; the install path documented in `README.md`.
- **Consumers:** agents working in the workspace; the user's skill directory
  after installation.
- **Dependency direction:** the skill depends on the repository's scripts and
  specs; nothing depends on the skill.
- **Release boundary:** versioned with this repository so the routes match the
  scripts they cite.

## 4. Behavioral model

**Routing table (the core of `SKILL.md`):**

| Situation | Route |
|---|---|
| "is the workspace healthy / why is the audit red" | `references/declaration.md` → `workspace_check.py --root .. --audit` |
| "add or fix a project declaration" | `references/declaration.md` → `init_project.py`, `schemas/project.schema.json` |
| "gate / evidence / release readiness" | `references/gate-and-evidence.md` → `driftwatchdog`, `docs/capabilities.md`, `promote_project.py` |
| "kind / profile / marker / secret vocabulary" | `references/vocabulary.md` → `vocabulary.json` |
| "rules, `.ai-rules`, handoff" | `references/rules-baseline.md` → `rules_drift.py`, `openspec/specs/rules-baseline/spec.md` |
| "shared library adoption (.NET/Rust)" | `references/platform-adoption.md` → `dotnet-platform-libs`, `rust-platform-libs` docs |
| "shared contracts / schemas" | `references/contracts.md` → `platform-contracts` |
| unknown / ambiguous | safe default: `workspace_check.py --root .. --audit` then `openspec list` |

**Reference discipline:** a reference MUST link to the owning script or spec and
MUST NOT restate policy; the check fails when a cited path or command is absent.

## 5. Contract and compatibility

- New files only; no schema, registry or checker change.
- The skill declares an explicit trigger description covering governance,
  declarations, profiles, gates, evidence, rules baseline, platform adoption and
  contracts, so it activates without the user naming it.
- Installation is documented, not automated: symlink or copy
  `skills/workspace-governance/` into the agent skills directory.

## 6. Failure and boundary policy

| Case | Result |
|---|---|
| Reference cites a missing path or command | `check_skill_refs.py` exits non-zero naming it |
| Skill or reference missing | check fails |
| Situation not covered by the routing table | route to the safe default and say so; never invent a mechanism |
| Skill instructs a destructive or shared-state action | forbidden; such actions stay behind existing explicit approval |

## 7. Verification oracle

- `python3 scripts/check_skill_refs.py` — every cited path and command exists.
- `python3 -m unittest discover -s tests -v` — all pass, including a negative
  test where a reference cites a missing script and the check fails.
- `python3 scripts/workspace_check.py --self-check` — unchanged, 0/0.
- `openspec validate --all --strict --no-interactive` — pass.

## 8. Decision ledger

**Assumptions:** agents discover skills by description; the repository is the
right owner because it owns the vocabulary and specs the routes cite.

**Resolved alternatives:**

- (a) One front-door skill vs one skill per concern → chose **one door**; the
  routing table is the value, and the user's stated preference is a single
  auto-triggering entry.
- (b) Restate policy in the skill vs link to specs → chose **link**; restating
  would create a second source of truth.
- (c) Ship to `~/.agents/skills` directly vs live in the repo + documented
  install → chose **repo + documented install**; the skill must be versioned
  with the scripts it cites.
- (d) Trust references vs check them → chose **check them**; a stale route is
  worse than no route.

**Deferred:** none.

**Blockers:** none.

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| R1 One auto-triggering door | discoverable entry | §4 table, §8(a) | `skills/workspace-governance/SKILL.md` | triggers on a governance situation | 1.2, 2.1 | skill exists with one trigger description |
| R2 Routes to references | per-concern detail without duplication | §4 | `references/*.md` | each situation routes | 2.2 | every route names a reference |
| R3 References stay valid | no drift from the repo | §4, §6, §8(d) | `check_skill_refs.py`, `tests/` | cited path exists / missing | 2.3 | check exits 0 |
| R4 Links, never restates | specs remain the source | §4, §8(b) | references | reference links a spec | 2.4 | no policy prose duplicated |
| R5 Installable | usable by agents | §5 | `README.md` | install documented | 2.5 | README install section present |
