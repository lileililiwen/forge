# Package Backlog

Index of the workspace-maturity packages. **Each package is an OpenSpec change
authored in the repository that owns the work**, so a fresh conversation
implements it there against that repository's `AGENTS.md`. This file is only the
index and the coordination rules; the proposal, design, tasks and spec live in
the change directory.

| #  | Package | Owning repository | Change location | State |
|----|---------|-------------------|-----------------|-------|
| 1  | `registry-and-declaration-integrity` | workspace-governance | `openspec/changes/archive/2026-09-26-registry-and-declaration-integrity/` | shipped |
| 2  | `shared-rules-single-source` | workspace-governance | `openspec/changes/archive/2026-09-26-shared-rules-single-source/` | shipped |
| 3  | `portfolio-vocabulary-normalization` | workspace-governance | `openspec/changes/archive/2026-09-27-portfolio-vocabulary-normalization/` | shipped |
| 4  | `gate-evidence-export` | driftwatchdog | `openspec/changes/gate-evidence-export/` | authored |
| 5  | `platform-consumer-bootstrap` | dotnet-platform-libs | `openspec/changes/platform-consumer-bootstrap/` | authored |
| 6  | `rust-platform-consumer-baseline` | rust-platform-libs | `openspec/changes/rust-platform-consumer-baseline/` | authored |
| 7  | `contracts-consumer-parity` | platform-contracts | `openspec/changes/contracts-consumer-parity/` | authored |
| 8  | `governance-front-door` | workspace-governance | `openspec/changes/archive/2026-09-27-governance-front-door/` | shipped |
| 9  | `post-mvp-readiness` | workspace-governance | `openspec/changes/archive/2026-09-27-post-mvp-readiness/` | shipped |
| 10 | `toolchain-decision-register` | workspace-governance | `openspec/changes/archive/2026-10-01-toolchain-decision-register/` | shipped |
| 11 | `shared-postgres-runtime-profile` | workspace-governance | `openspec/changes/archive/2026-10-01-shared-postgres-runtime-profile/` | shipped |
| 12 | `standalone-project-boundary` | workspace-governance | `openspec/changes/archive/2026-10-01-standalone-project-boundary/` | shipped |
| 13 | `post-mvp-readiness` (propagated, per repository) | each product repository | `openspec/changes/post-mvp-readiness/` in 59 repositories | parked — campaign, not yet started |
| 14 | `python-security-audit-scoping` | workspace-governance | to be authored | queued — decision D8 accepted 2026-10-17 |

Package 4 was called `gate-run-evidence` in the original portfolio map; 5 was
`platform-adoption-dotnet`; 6 `platform-adoption-rust`; 7
`contracts-single-source`. The authored changes use the more precise names
above, because reading the target repositories showed the real gap is narrower
than the map assumed (for example driftwatchdog's `release-evidence-gate` spec
already owns the required-evidence and stale-revision guards, so package 4 is
the *export*, not the guard).

Packages 9–12 are docs-only or governance-internal changes owned by
`workspace-governance`. Package 9 ships the post-MVP documentation and
terminal capture; package 10 ships the `decisions.json` register; package 11
adds the external-PostgreSQL runtime template family; package 12 adds the
repository-relative standalone classification. None of them touches a
sibling repository, runtime, or promotion ledger.

Package 13 is a **campaign, not one change**: package 9 shipped the post-MVP
documentation and terminal-capture contract in `workspace-governance`, and its
package was then propagated as a per-repository change directory into **59**
product repositories, where it stands at **0 of ~1,324 tasks** and has never run.
Measured on 2026-10-17: every one of the 59 targets a `readiness` capability that
is **absent from that repository's canonical specs**, so archiving them without
doing the work would promote undelivered requirements — which is why the owner's
recorded decision is to keep them parked and run them as one campaign rather than
to archive or delete them. A campaign run closes each repository's package with
the work actually done, then archives it normally (no `--skip-specs`), one
reviewable commit per repository, ordered by blast radius.

## Implement a package

1. Open the owning repository.
2. `openspec list` — that repository holds exactly one active change for the
   package; implement one change at a time.
3. Read the change's `proposal.md`, `design.md`, `tasks.md` and
   `specs/<capability>/spec.md`.
4. **Read `## 0. Caveats and confirmations` in `tasks.md` first.** Every change
   records there the non-obvious intent and the decisions a reviewer must confirm
   before implementing; the task groups below it assume those caveats are read.
5. Follow that repository's `AGENTS.md`: BFS → DFS → BFS, two commits
   (implementation + archive, then a `HANDOFF.md`-only commit), strict OpenSpec
   validation, no push.
6. Never implement a cross-repo package from another repository.

## Dependency order

- Shipped: 1, 2, 3, 8, 9, 10, 11, 12.
- 4 depends on 3 (it exports in the vocabulary 3 canonicalized).
- 5 and 6 are independent of each other and of 3.
- 7 depends on 5 and 6 (it names their consumer wiring as the follow-up).
- 9–12 are governance-internal and have no cross-repo dependencies.

## Coordination rules

- Consume the governance vocabulary; never re-declare a field name, a state, a
  `kind` or a `profile` locally.
- One repository per change; author the change where the work is owned.
- If a package reveals a second lifecycle, owner or acceptance oracle, stop and
  propose a follow-up package with a dependency edge rather than expanding it.
- Portfolio oracle after any package:
  `python3 scripts/workspace_check.py --root .. --audit --json` in
  `workspace-governance` must stay at 0 errors. The 211 errors / 17 warnings
  seen today are pre-existing sibling-repo metadata drift, not a regression
  from any shipped package; see the "Portfolio impact observed" section of
  `HANDOFF.md` for the per-package attribution.
