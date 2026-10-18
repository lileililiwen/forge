# Proposal: Spec Purpose placeholders are backfilled once, then blocked

## Why

Archiving a change writes a `## Purpose` placeholder into every spec file the
change produced. The archiver is third-party (`@fission-ai/openspec`, vendored
under `pluggify/node_modules`), so the defect cannot be fixed where it is born —
it has to be fixed by a tool this repository owns and a rule that stops the next
one from appearing.

**Measured across the fleet on 2026-10-03.** 79 repositories hold 1,958 spec
files under `openspec/specs/<capability>/spec.md`. **1,234 of them still hold
the marker**, across 71 repositories — 1,229 in the current form and 5 in the
older short form the archiver used before it learned to append
`Update Purpose after archive.`:

```markdown
## Purpose
TBD - created by archiving change work-on-main-baseline-rule. Update Purpose after archive.
```

The marker must be the whole `## Purpose` **section** body, not merely a line
starting with it, so a real sentence appended after the marker is a real purpose
and never a finding; 7 of the 1,234 carry a blank line between the heading and
the marker, and a section-strict pattern matches exactly the same 1,234 files
fleet-wide that a looser line pattern does, so the strictness is free.

1,161 of the 1,234 name a change whose archive directory still exists, and
1,159 of those carry a real `## Why` section of prose in the change's own
`proposal.md`. The marker is therefore not information the fleet lost: it
is information the fleet already wrote down one directory away and never
promoted. 1,089 clear the 120-character floor this change adopts and become the
resolved Purpose; the remainder fall back to the derived sentence. Only 77
markers have no recoverable `## Why` — 75 because the archived proposal
genuinely has no `## Why` heading, 6 because the archive directory has no
`proposal.md`, and 4 because no archive directory exists at all. 2 change names
are ambiguous, appearing under two archive directories each, and must be refused
rather than guessed.

**The defect is invisible to the audit.** No rule reads `## Purpose`, so a spec
whose purpose is literally the string `TBD` reads as a finished spec. The
archiver also leaves the *live* change directories alone, so the same defect is
still being produced for every change archived from now on.

**The prior art is not shared and not sufficient.**
`openlearning/scripts/fill_spec_purposes.py` (committed there as `1eac6d1`)
implements a mechanical derivation and was run in that one repository only. It
is a single-repository script: it resolves `ROOT` from its own location, walks
one `openspec/specs` tree, has no notion of the archive, and offers no plan
review before writing. It also has no dry-run-by-default and no drift refusal,
so it is not a fleet tool and could not be run against 79 repositories
unsupervised.

**The gate wiring this needs does not exist yet.** Owner ruling D18 states that a
verifier no gate runs does not gate. The obvious candidate — the
`governance-quality` concern in `driftwatchdog`, which already runs
`workspace_check.py` and parses its JSON envelope — **cannot** carry this rule as
written. Its effective code scope is the closed allow-list
`GOVERNANCE_QUALITY_DECLARED_CODES`
(`driftwatchdog/src/gate/governance/mod.rs:73-78`), which holds four codes and
does not include this one; the adapter drops any finding whose code is outside
the effective scope (`mod.rs:453`), and a manifest that *names* an undeclared
code is refused as `REVIEW_REQUIRED` before the command even starts
(`mod.rs:292-313`). Worse, the checker's exit 1 is an *expected* exit for that
concern, so an out-of-scope ERROR yields `PASS` rather than a failure: enabling
this rule behind that concern alone would produce a check that runs, fails, and
is reported green. See `design.md` §6.

## What Changes

- **Ship one shared backfill tool**, `scripts/spec_purpose_backfill.py`, owned by
  this repository and promoted from the `openlearning` prior art. It prefers the
  archived change's own `## Why` prose and falls back to the requirement-derived
  sentence, records which source each row used, defaults to a dry run, writes
  only under `--apply`, and refuses to plan when the fleet has drifted from the
  committed plan.
- **Declare `SPEC_PURPOSE_PLACEHOLDER` at ERROR** in `scripts/workspace_check.py`,
  naming the offending spec file and the change it was archived from.
- **Fence the rule exactly as the taxonomy rule is fenced** — a committed,
  repo-relative constant `SPEC_PURPOSE_AUDIT_ENABLED = False`, never an
  environment variable, never a second mechanism — with a test proving that
  flipping it on turns the same unmigrated corpus red.
- **Record the gate-wiring gap as a blocker** rather than a claim. The
  `governance-quality` concern's declared code set must be widened before this
  rule is enabled, and that edit is in a sibling repository this change is not
  authorized to make.

## Package Boundary and Split Assessment

This is one package because the tool and the rule are two halves of one
sequencing decision: the rule is worthless without the tool, and the tool is a
fleet-wide cross-repo source edit that only the owner may authorize. Shipping the
rule without the tool would ship a check nobody can clear; shipping the tool
without the rule would leave the archiver producing new placeholders into a
corpus with no measurement.

| Package | Single outcome | Owner / language | Boundary and contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `spec-purpose-backfill-gate` (this) | Every spec `## Purpose` is real prose, and a new placeholder blocks | `workspace-governance`, Python 3.11 stdlib | `scripts/spec_purpose_backfill.py`, `SPEC_PURPOSE_PLACEHOLDER`, `SPEC_PURPOSE_AUDIT_ENABLED` | the `## Purpose` heading and the `### Requirement:` heading the archiver already emits | 1,234 markers measured across 71 of 79 repositories; 1,089 clear the `## Why` floor |
| `driftwatchdog` gate wiring (**not in this change**) | The new code is inside the concern's declared set so a real ERROR fails a real gate | `driftwatchdog`, Rust | `GOVERNANCE_QUALITY_DECLARED_CODES` | the JSON envelope this change adds a code to | the adapter's own filter test |
| `axis_migration` (shipped) | Nothing — reused only for its tool shape | `workspace-governance` | dry-run default, `--apply`, drift refusal, committed plan | the fleet's `.project.json` files | already green |

## BFS Impact Map

- **Actors:** the shared-infrastructure owner (runs the backfill under
  authorization), every repository that archives a change (produces the
  placeholder), the governance auditor (reports it), the driftwatchdog gate
  runtime (would carry it, and cannot yet).
- **State:** a spec moves `placeholder → resolved`, and the resolution is either
  the archived change's own prose or a sentence derived from the spec's first
  `### Requirement:`. Nothing else in the spec is touched.
- **Contracts:** `openspec/specs/<capability>/spec.md` under each repository is
  rewritten in exactly one place. No declaration, registry entry, schema, or
  capability binding is affected, so `id` immutability and the axis-split
  invariants are untouched.
- **Data integrity:** the archive directory is resolved by stripping the
  `YYYY-MM-DD-` prefix from a directory name and matching the placeholder's
  change name, because the placeholder carries the **bare** change name while the
  archive directory is date-prefixed. An ambiguous name is refused, never
  guessed. A `## Why` shorter than the recorded minimum is treated as absent so
  a one-word section is not promoted to a purpose.
- **Failure and boundary:** a repository with no `openspec/specs` tree is
  skipped. A spec with no `### Requirement:` heading falls back to the
  capability-name sentence. A missing or unreadable `proposal.md` falls back
  too. No network, no project command, no file outside the repository's own
  `openspec/` tree is read.
- **Compatibility:** the fallback derivation is preserved byte-for-byte from the
  `openlearning` script so the fleet write is reproducible against the result
  that script already produced in its one repository.
- **Security:** both sources are Markdown the owner already committed. The tool
  writes only files it has planned and only under `--apply`.
- **Tests:** placeholder detection for both placeholder generations; the
  requirement-derived fallback reproducing the prior art's exact string; the
  `## Why` preference; ambiguous-archive refusal; dry run writing nothing; the
  `--apply` reminder naming the flag; the committed-constant fence refusing an
  environment variable; and the fence-open case reporting ERROR and turning the
  gate red on the same corpus.
- **Dependencies:** Python 3.11 standard library only. No third-party import.

## Capabilities

- **New:** `spec-purpose-integrity`.
- **Modified:** none. `portfolio-vocabulary` is not extended, no new vocabulary
  key is added, and no existing finding code is renamed, narrowed, or reused.

## Non-goals

- No change to any sibling repository's source. The backfill is a fleet-wide
  cross-repo source edit and is applied by a separately authorized step; this
  change ships the tool and the rule only.
- No patch to `@fission-ai/openspec` and no vendored-archiver change. The
  archiver is third-party; the durable fix is the tool plus the rule.
- No change to the `## Purpose` heading's shape, and no new section in the spec
  template.
- No second fencing mechanism. `SPEC_PURPOSE_AUDIT_ENABLED` is the only hold, and
  the severity is never softened to WARNING to achieve it.
- No environment-variable escape, no skip flag, and no exemption registry.
- No change to `driftwatchdog`. The code-scope widening that D18 requires is a
  separate, separately authorized cross-repo step.
- No backfill of a placeholder that has no `## Why`, beyond the existing
  requirement-derived fallback, and no new source invented for it.

## Owner rulings — decided

1. **`SPEC_PURPOSE_PLACEHOLDER` is ERROR**, and the whole spec-purpose audit is
   held off the fleet until the backfill has been applied, under a committed
   constant. The severity is *not* softened to WARNING to achieve that. This is
   the taxonomy rule's shape, verbatim, and the reason is the same: 1,234
   violations exist today, so enabling the rule first would turn the audit red
   for everyone with no recorded owner action.
2. **The requirement-derived fallback is preserved byte-for-byte.** It is
   judgment-free, which is the only property that makes a 1,234-file write
   reviewable. Three changes were made, and none of them alter the derived
   string for a spec that has a `### Requirement:` heading: the source is now
   read and recorded per row, the change-name → archive-directory resolution was
   added (the prior art had none), and both placeholder generations are
   detected. `design.md` §4 states exactly what changed.
3. **An ambiguous archive name is refused, not resolved.** Two repositories hold
   a change name under two archive directories each. Picking the newest would be
   a guess about which change the spec came from, and the row is a published
   sentence. The tool refuses to plan and names both directories.
4. **The gate-wiring gap is a blocker, not a footnote.** D18 is a standing
   ruling and this change does not satisfy it. `design.md` §6 records the
   verification, the exact file and line that must change, and the fact that the
   check would otherwise be reported green while failing.
