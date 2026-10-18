# Tasks: One front-door skill that routes to the governance references

## 0. Caveats and confirmations

- [ ] **Package 3 must land first.** The vocabulary reference links
  `vocabulary.json`, which package 3 (`portfolio-vocabulary-normalization`)
  creates. Implement 3 before 8, or the reference check fails on a missing path.
- [ ] **Build the reference check before the references.** The check is what
  proves the routes; add it early so each reference is validated as written.
- [ ] **Links, never restatements.** A reference that copies a requirement out of
  a canonical spec creates a second source of truth and will drift; link the spec
  instead.
- [ ] **One door only.** Do not add a second skill per concern; the routing
  table's value is that it is the single entry point.
- [ ] **No destructive instructions.** The skill must not instruct pushing,
  deploying or promoting; those stay behind the existing explicit-approval
  boundary.
- [ ] **Installation is documented, not automated.** The skill lives in this
  repository, versioned with the scripts it cites; `README.md` documents the
  install path.
- [ ] **Stage only this change directory.** The pre-existing uncommitted
  `.project.json` and untracked `MATURITY_PROGRESS_SUMMARY.md` must not be swept
  into the commits.

## 1. BFS — Baseline and impact coverage

- [ ] Inventory every governance entry point (scripts, specs, docs) and the
  situations that select them; record the routing table as before-evidence.
- [ ] Confirm no existing skill or document already routes these concerns.
- [ ] Add the reference-integrity test skeleton (including a negative case).
- [ ] Verify the implementation-handoff gate: Markdown + Python 3.11,
  `workspace-governance`, no shared-library boundary.

## 2. DFS — Requirement-by-requirement implementation

- [ ] R1: write `skills/workspace-governance/SKILL.md` with one broad
  auto-triggering description and the routing table.
- [ ] R2: write the six references (`declaration`, `gate-and-evidence`,
  `vocabulary`, `rules-baseline`, `platform-adoption`, `contracts`), each
  linking to the owning script or spec.
- [ ] R3: implement `scripts/check_skill_refs.py` (extract cited paths and
  `scripts/*.py` commands; fail on a missing one) and its test.
- [ ] R4: ensure no reference restates policy; each links the canonical spec.
- [ ] R5: document installation in `README.md`.

## 3. BFS — Cross-surface regression and completeness

- [ ] Confirm every route in the table resolves to a real reference and every
  cited command runs `--help` successfully.
- [ ] Confirm the negative test fails when a reference cites a missing script.
- [ ] Confirm the skill instructs no push, deploy, or promotion action.
- [ ] Confirm the safe default is stated for an unknown situation.

## 4. Verification

- [ ] `python3 scripts/check_skill_refs.py`.
- [ ] `python3 -m unittest discover -s tests -v`.
- [ ] `python3 scripts/workspace_check.py --self-check` — unchanged, 0/0.
- [ ] `node scripts/check-openspec-change-names.mjs` and
  `openspec validate --all --strict --no-interactive`.
- [ ] Record actual outputs; report any environment-blocked check with its exact
  next action rather than as passing.
