# Design: record the archived governance origin and Forge maintenance owner

## Implementation boundary

Update only Forge-owned `HANDOFF.md`, `ROADMAP.md`,
`docs/workspace-governance-import.md`, the canonical
`forge-workspace-governance-in-tree` spec, and this OpenSpec change. Do not
modify the archived source repository or the imported module's source files.

## Language and runtime

Markdown and OpenSpec only. No build or test toolchain changes. Verify with
`openspec validate --all --strict --no-interactive`, `git diff --check`, and
the required local Forge Gate.

## Ownership and shared code

Forge owns its in-tree copy at `modules/workspace-governance/`. The original
source repository is archived by user-confirmed external state. Maintainers
make future source changes in Forge and preserve the import's source-only
history. Sibling projects consume Forge interfaces and can migrate separately;
they do not determine whether the source origin is archived.

## User experience and interface

No product UI or CLI surface changes. Maintainers reading Forge's records see
one consistent source owner and the actual archive status.

## Behavioral model

| Concern | Recorded state | Consequence |
|---|---|---|
| Original source origin | Archived | No further maintenance is directed there |
| Forge module copy | Maintained under `modules/workspace-governance/` | Needed implementation changes are made and verified in Forge |
| Source-size conformance | Remaining Forge quality work | Does not block the already completed origin archive |
| Sibling portability | Separate remaining consumer work | Does not reopen or unarchive the origin |

## Contract and compatibility

No runtime or metadata schema changes. The canonical requirement updates the
existing in-tree ownership capability. Historical import revision, commits,
and archive evidence remain intact.

## Failure and boundary policy

This change relies on the user's direct confirmation that the source origin is
archived. It does not infer or execute remote archive state. If later evidence
contradicts that status, update Forge's record through a new reviewed change;
do not write to the external origin as part of this package.

## Verification oracle

- Strict validation promotes the changed canonical requirement.
- Current Forge records identify the same archived origin and in-tree owner.
- A focused search finds no current statement that origin archival awaits
  source-size conformance or sibling portability.
- `git diff --check` passes. The bounded full Gate was run twice and remains
  BLOCKED by `declared-verification` (exit 101), while `tests` and the other
  checks pass. Running its exact declared command alone passes. This
  documentation-only change does not alter either Cargo suite; retain the
  blocked verdict and remediation in HANDOFF rather than claiming a green Gate.

## Decision ledger

- Trust the user's archive confirmation as the authoritative input for this
  correction.
- Retain source-size conformance because Forge still benefits from applying
  its baseline to the imported project; remove only the false retirement
  dependency.
- Keep sibling portability and origin archive status as separate facts.
