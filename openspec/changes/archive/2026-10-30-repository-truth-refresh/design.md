## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The four status documents | `forge` | Yes |
| The governance checker | `forge` | Yes |
| `.project.json` metadata | `forge` | Yes |
| The Doctor profile model | `forge`, proposed follow-up | No |

## The Count Rule

The checker already computes the active-change set for the pointer rule. The
count rule reuses that directory walk:

```text
archived = directories under openspec/changes/archive (not hidden)
specs    = directories under openspec/specs (not hidden)
for each of README.md, ROADMAP.md, docs/requirements-coverage.md,
openspec/config.yaml:
  every "N archived ... changes" claim must equal archived
  every "N ... canonical specs" claim must equal specs
```

Claims are matched by their wording, not by a fixed line, so a document may
restate the sentence as long as the number is right. A missing claim is not an
error: a document may describe the repository without a count. A wrong claim is
an error naming the file, line, stated value and computed value.

The rule runs in the same script as the pointer, placeholder and link rules, so
CI's `governance` job and a local invocation see it without a new workflow step.

## Why Not Generate the Counts

Writing the numbers into four prose files from a generator would remove the
drift but also remove the human-readable sentence. A checker keeps the prose
and makes the wrong value fail, which is the repository's existing pattern for
placeholders, links and status claims.

## The Metadata Corrections

| Field | Before | After | Why |
|---|---|---|---|
| `discoverability.visibility` | `private` | `public` | `gh repo view` reports `visibility: PUBLIC` |
| `verification.evidence_status` | `planned` | `verified` | the declared command and the full Gate run every cycle; the advisory exception is now recorded |
| `verification.gate_runtime` | `driftwatchdog` | `forge` | the bundled engine is the runtime; `forge-native-gate` keeps historical values readable but new declarations identify Forge |

## The Native-Profile Gap

`forge.yaml` declares `profile: rust-product`, which is the *governance*
profile attached to the `rust-web` descriptor. `doctor` looks up Forge profile
descriptors, so it reports `profile-known` FAIL and leaves descriptor-gated
checks unavailable; `register` refuses for the same reason.

A correct fix decides what a native project is: a known non-generatable
profile, or a Doctor path that reads the governance profile from
`.project.json`. Both are product decisions. `ROADMAP.md` records the proposed
`native-project-profile` package and its question instead of forcing a
half-answer here.

## Failure Boundaries

- A count claim edited without a matching directory change fails the rule.
- A directory added without updating the four documents fails the rule.
- The metadata edits do not change runtime behaviour; `cargo test` and the Gate
  verify that.

## Verification Strategy

1. `node scripts/check-spec-governance.mjs` passes with the updated counts.
2. A deliberate wrong count fails the check naming the file and both numbers
   (mutate, observe, restore).
3. `cargo test` and `forge gate` still pass; `.project.json` parses under the
   workspace schema.
4. `openspec validate --all --strict --no-interactive` passes.
