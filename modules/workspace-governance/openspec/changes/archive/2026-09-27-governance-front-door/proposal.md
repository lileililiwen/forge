# Proposal: One front-door skill that routes to the governance references

## Why

The portfolio now has a dozen governance entry points an agent must choose
between correctly: `workspace_check.py` (`--audit` / `--self-check` /
`--project`), `auto_gate.py`, `jenkins_manifest.py`, `product_code_quality.py`,
`rules_drift.py`, `maturity_audit.py`, `promote_project.py`, the Forge adapter,
`init_project.py`, and the six canonical specs. Nothing tells an agent which to
run for a given situation, so two failure modes recur:

- running the wrong command (for example `maturity_audit.py` where
  `workspace_check.py --audit` is authoritative — exactly the divergence
  packages 1 and 2 removed);
- not knowing which of the many rules applies, and inventing a new mechanism
  beside an existing one.

The repository already owns the vocabulary and the specs; it lacks the single
auto-triggering entry that routes to them.

## What Changes

- Ship `skills/workspace-governance/SKILL.md` — one broad, auto-triggering
  skill that recognises any governance situation and routes to a per-concern
  reference.
- Ship `skills/workspace-governance/references/*.md`: `declaration.md`,
  `gate-and-evidence.md`, `vocabulary.md`, `rules-baseline.md`,
  `platform-adoption.md`, `contracts.md`, each pointing at canonical scripts and
  specs rather than restating policy.
- Add `scripts/check_skill_refs.py` (or a test) that fails when a reference
  names a command or path that does not exist, so the skill cannot drift from
  the repository.
- Document installation into the agent skills directory in `README.md`.

## Package Boundary and Split Assessment

**Single outcome:** one discoverable entry that routes correctly to existing
governance mechanisms.

**Included:** `skills/workspace-governance/**`, one reference-integrity check,
`tests/`, `README.md`.

**Excluded:** changing any checker, any policy, any canonical spec, or any
other repository; shipping a second skill per concern (the point is one door).

**Split signals considered:** the routing table and the reference files share
one owner, one lifecycle (docs), one contract (the file paths and commands
cited) and one oracle (reference check passes). Splitting them would create a
door without rooms.

**Dependencies:** `registry-and-declaration-integrity`,
`shared-rules-single-source` (shipped) and `portfolio-vocabulary-normalization`
(for the vocabulary reference). It is the last package in the map because it
routes to the others.

**Package map:** package 8 of 8.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| workspace-governance `AGENTS.md`, `README.md` | governance ownership statement; mandatory-command block | the command inventory | prose, not routeable; no per-concern entry | workspace-governance | **extend shared owner** |
| canonical specs | `openspec/specs/*/spec.md` (6) | authoritative per-capability contracts | not linked from any single entry | workspace-governance | **adopt** (reference, never restate) |
| user skill directory | `~/.agents/skills/<name>/SKILL.md` | agent auto-trigger convention | lives outside the repo, so it must be installed | user environment | **adapt through a generic adapter** (documented install) |
| forge | `forge governance use workspace-governance` | control-plane integration | different surface (control plane, not an agent skill) | forge | **not applicable** |
| all other repos | — | none | no skill machinery is shared | each product | **not applicable** |

## BFS Impact Map

- **Capabilities:** agent routing; reference integrity.
- **Consumers:** any agent working in the workspace; `README.md` readers.
- **Contracts:** the skill's trigger description; the reference file names; the
  install path.
- **Failure and boundary:** a reference to a missing file or command → the
  integrity check FAILS; an unknown situation → the skill routes to
  `workspace_check.py --self-check` as the safe default rather than guessing.
- **Tests:** reference-integrity check over every cited path and command.
- **Dependencies:** documents only; no runtime dependency.
- **Compatibility, privacy, security:** the skill must not instruct pushing,
  deploying, or promoting; destructive or shared-state actions stay behind the
  existing explicit-approval boundaries.

## Capabilities

- `governance-front-door`.

## Non-goals

- No second skill per concern, and no duplication of spec content into the
  skill (references link, specs own).
- No change to any checker, policy, or canonical spec.
- No automatic installation; installation is documented and manual.
- No new governance authority.
