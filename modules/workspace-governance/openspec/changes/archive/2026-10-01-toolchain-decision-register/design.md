# Design: decisions.json register, fail-closed loader, and check wiring

## 1. Implementation boundary

- Repository: `workspace-governance` only.
- New files: `decisions.json`, `scripts/decisions.py`,
  `docs/toolchain-decisions.md`, `tests/test_decisions.py`.
- Changed files: `scripts/workspace_check.py` (self-check integration +
  `--decisions` flag + portfolio-audit finding for an absent register),
  `docs/capabilities.md` (D3 ownership note), `README.md` (one line).
- Must not change: `vocabulary.json` keys, `profiles/*.json`, registry
  content in `projects.json`, promotion ledger semantics, jenkins manifest,
  runtime templates, any sibling repository.

## 2. Language and runtime

- Python 3.11, stdlib only (json, re, pathlib, dataclasses), consistent
  with every other governance script.
- Tests: `python3 -m unittest discover -s tests`.
- Validation commands: `python3 scripts/workspace_check.py --self-check`;
  `node scripts/check-openspec-change-names.mjs`;
  `openspec validate --changes --strict --no-interactive`.
- Target: local execution, no network, matches the repo's no-daemon
  posture.

## 3. Ownership and shared code

- Governance owns the register (root HANDOFF: "Workspace Governance owns
  policy/declarations"). Consumers read the file or, later, the forge shim;
  nobody else writes it.
- Dependency direction: `workspace_check.py` and future adapters →
  `scripts/decisions.py` → `decisions.json`; the loader depends only on
  stdlib and receives `projects.json` data as an argument (no circular
  registry read inside the loader).
- Release boundary: register format versioned by `"version": 1` field; a
  shape change is a new governance OpenSpec change, consumers pin the
  version they read.

## 4. Behavioral model

`decisions.json` shape:

```json
{ "version": 1,
  "decisions": [ {
      "id": "D2", "title": "Self-hosted identity provider",
      "status": "accepted", "date": "2026-09-30",
      "owner_project": "workspace-governance",
      "decision": "<one paragraph>",
      "policy_boundary": "<restated public-site exclusion for D2>",
      "downstream": [ { "repo": "jenkins-local",
                        "change": "identity-runtime-profile" } ] } ] }
```

| Rule | Enforcement at load time |
|---|---|
| `id` unique, matches `^D[0-9]{1,3}$` | `DecisionError` naming entry |
| `status` ∈ {proposed, accepted, rejected, superseded} | `DecisionError` |
| `date` ISO-8601 | `DecisionError` |
| `owner_project` and every `downstream[].repo` ∈ registry ids | caller-supplied registry set ⇒ `DecisionError` listing offenders |
| `downstream[].change` kebab-case (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`) | `DecisionError` |
| empty `decisions` array | valid (register predates entries) |
| supersede chain: `status: superseded` entries keep full text; superseding entry id is new; no id reuse ever | loader id-uniqueness rule |

State transitions are edits under git review; the immutable-after-accept
rule is policy (docs + review gate), not code, because detecting history
rewrites from a script means diffing git, which is out of scope (ledger L3).
`workspace_check.py --decisions` prints `{id,title,status,date}` per entry
plus a total, JSON when `--json`, and exits 1 on loader error.

## 5. Contract and compatibility

- Public surface: `scripts/decisions.py` — `load_decisions(path:
  Path) -> list[Decision]` (raises `DecisionError`), `Decision`
  dataclass, `validate_against_registry(decisions, registry_ids:
  set[str]) -> list[str]`. Additive import for any future consumer.
- File is the wire contract: consumers pin `version: 1`; unknown top-level
  keys ⇒ `DecisionError` (fail-closed inherits vocabulary discipline);
  unknown keys inside an entry ⇒ allowed-but-recorded (forward
  compatibility for additive fields), decided in ledger L4.
- Env keys: none. No secrets in the register (validated by the existing
  secret-substring scanner when run over governance files — self-check
  already covers root JSON).

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| `decisions.json` missing, in governance self-check | ERROR finding `DECISIONS_REGISTER_MISSING`, non-zero |
| Missing, in portfolio audit of other repos | single WARNING `DECISIONS_REGISTER_ABSENT`; per-project checks unchanged |
| Malformed JSON / bad enum / bad id pattern | loader raises `DecisionError` naming file, entry index, field; `--decisions` exits 1 |
| Unknown `owner_project`/downstream repo | `DecisionError` listing offenders |
| Duplicate ids | ERROR |
| Accepted entry edited in place (title/decision/status flip to weaker state) | not detectable by this tool; docs name the required review behavior (supersede via new entry) |

## 7. Verification oracle

- `tests/test_decisions.py` (unittest, tmp fixtures): valid register loads;
  empty array loads; missing file raises; duplicate id raises naming both
  indices; bad status/date/regex raises with field name; unknown registry
  reference raises with offender list; `--decisions --json` shape asserted;
  forward-compat entry key tolerated.
- Seeded content assertion: the four real entries (D1–D4) parse, are
  `accepted`, cross-reference registry ids that exist today
  (`workspace-governance`, `forge`, `platform-contracts`, `openpanel`, …),
  and D2's `policy_boundary` contains the phrase excluding the public
  catalog from shared login (string assertion — drift-detectable).
- `python3 scripts/workspace_check.py --self-check` — 0 errors with the
  register present, and one deterministic ERROR when a temp copy removes it.
- Full portfolio audit (`--root .. --audit --json`) must show the new codes
  only in the expected sections; no new project-level errors.
- Evidence before any task box ticks: commands + outputs recorded in
  HANDOFF per repo cadence.

## 8. Decision ledger

| # | Decision | Choice | Alternatives rejected |
|---|---|---|---|
| L1 | Home | sibling `decisions.json` + own loader | new `decisions` key inside vocabulary.json (breaks REQUIRED_KEYS contract, mixes enumeration with lifecycle) |
| L2 | Absent register | self-check ERROR, portfolio WARNING | hard failure everywhere (would break 77-project audit runs mid-migration) |
| L3 | Immutability | review gate + supersede convention in docs | git-diffing enforcement (scope creep, false security) |
| L4 | Unknown entry keys | tolerated; unknown top-level keys fatal | all-unknowns fatal (blocks additive evolution before version 2 exists) |
| L5 | Seeding | transcribe D1–D4 verbatim from `TOOLCHAIN_ROADMAP.md` with downstream change names already planned | inventing extra decisions |
| L6 | Forge shim exposure | deferred to a forge-owned change | extending `forge_governance_adapter.py` here (assessment: it is read-only stdin/stdout per governance-provider-contract v0.1.0; adding a method there without the contract change violates the boundary) |
| — | Blockers | none | — |

## Traceability

| Req | Proposal outcome | Design section | Boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| REG-001 register exists, fail-closed | What Changes #1 | §3 §4 §5 §6 | decisions.json, scripts/decisions.py | valid/empty/malformed/missing | 2.1–2.2 | §7 loader tests |
| REG-002 entries cross-reference the registry | What Changes #1 | §4 §6 | validate_against_registry | success/failure | 2.2 | §7 offender tests |
| REG-003 lifecycle: supersede, no id reuse, statuses | What Changes #1 | §4 §8 | loader rules + docs | transition/boundary | 2.2, 2.4 | §7 duplicate+seed tests |
| REG-004 check wiring + human doc + D3 note | What Changes #2–3 | §1 §6 §7 | workspace_check.py, docs | self-check + --decisions | 2.3, 2.5 | §7 audit-code tests |
