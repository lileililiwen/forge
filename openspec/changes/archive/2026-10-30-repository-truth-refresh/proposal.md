# Change: repository-truth-refresh

## Why

Four status documents disagree with the repository they describe. The actual
counts are **225 archived changes and 172 canonical specs**; the documents say:

| File | Claim |
|---|---|
| `openspec/config.yaml` | 118 archived / 94 specs |
| `docs/requirements-coverage.md` | 129 / 103 |
| `ROADMAP.md` | 155 / 113 |
| `README.md` | 167 / 121 |

This is the second recorded drift of the same literals: `CHANGELOG.md` says an
earlier change refreshed them. They are hand-maintained and nothing checks
them, so they will drift again.

`.project.json` has the same problem in three fields: it declares
`visibility: private` although the GitHub repository is public, records
`evidence_status: planned` although the suite and Gate run every cycle, and
names `gate_runtime: driftwatchdog` although Forge owns the bundled engine.

One further truth gap is left open on purpose: `forge doctor .` reports
`profile-known` as FAIL because `rust-product` is a governance profile, not an
MVP descriptor, and `forge register .` therefore refuses. Fixing that needs a
native-project profile decision, not a metadata edit.

## What Changes

- **Counts become checked.** `scripts/check-spec-governance.mjs` computes the
  archived-change and canonical-spec counts from the directories and fails when
  any of the four documents states a different number, naming the file, line
  and both values.
- **The four documents state the real counts** (225 / 172 at this change).
- **`.project.json` states the real facts**: `visibility: "public"`,
  `evidence_status: "verified"`, `gate_runtime: "forge"`.
- **The native-profile gap is recorded, not hidden.** `ROADMAP.md` names the
  proposed `native-project-profile` package and what it must decide, so the
  next cycle does not rediscover it.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `scripts/check-spec-governance.mjs` | new count rule |
| `README.md`, `ROADMAP.md`, `docs/requirements-coverage.md`, `openspec/config.yaml` | counts refreshed; proposed follow-up named |
| `.project.json` | visibility, evidence status and runtime |
| `openspec/specs/specification-governance-refresh/spec.md` | one added requirement |

Persistence: none in Forge. The registry is untouched.

Failure boundaries: a stale count fails the governance check and CI's
`governance` job; a renamed archive or spec directory changes the computed
number and therefore the expected literal.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `specification-governance-refresh` | modified | deterministic count check |
| `artifact-and-ci-baseline` | read only | the check runs in the governance job |

## Non-Goals

- No change to the Doctor profile model, `forge register` or `forge check`.
  That is the proposed `native-project-profile` change.
- No rewrite of historical HANDOFF entries. They stay as historical evidence.
- No release, tag or push.
