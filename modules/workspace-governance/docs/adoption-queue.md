# Adoption Queue

## Adopted in the first coordination slice

These projects now have `.project.json`, a selected governance profile, and a real verification command:

1. `forge` — portfolio control-plane candidate.
2. `driftwatchdog` — shared quality/evidence gate runtime.
3. `sisyphusfy` — agent iteration runtime.
4. `ariadex` — supervised runtime operator.
5. `dotnet-platform-libs` — reusable .NET library platform.
6. `labrys` — application/runtime platform candidate.
7. `jenkins-local` — deployment execution layer.
8. `alltools-platform` — application-specific platform.

## Second batch adopted and promoted

The next deployment-capable batch now has declarations, profile checks,
and a recorded promotion in `deployment/promotions.json`:

- `alethefy`
- `crossify`
- `cvunify`
- `hermexa`
- `hypora`
- `lexora`
- `openaccounting`

Each project also carries a `release_evidence` block in `configured`
state pointing at pilot-context placeholder artefacts (the promotion
ledger records the actual decision; artefacts will be replaced when the
project's release pipeline produces them).

## Third batch adopted

- `hermora`
- `hestia-lab`
- `lore-mix`
- `metiscope`
- `mortalect`
- `opendockify`

These projects are also structurally adopted and remain non-deployable pending runtime evidence.

## Fourth batch adopted

- `actoria`
- `argoset`
- `byok-translator`
- `mnemyx`
- `openpanel`
- `pluggify`

These projects are structurally adopted and remain non-deployable pending runtime evidence.

## Large clean-worktree batch adopted

The following clean repositories were adopted in one guarded wave:

`aegify`, `aesthra`, `arivio`, `bargora`, `cargo-audit-boost`,
`cargo-matrix-boost`, `cargo-sonar`, `catchen`, `chinago`, `fotofy`,
`gradevia`, `kairosa`, `linglitch`, `namevia`, `orphevia`, `pangoman`,
`ploutify`, `reelio`, `rostera`, `rust-ecommerce`, `shopifireman`,
`singleatee`, `smotoox`, `somodanote`, `stylify`, `sutravia`, `tantalyn`,
`testify`, `transitfy`, `triplanify`, `trippify`, and `ziban`.

Dirty, locked, and non-Git repositories were not modified.

## Remaining work

The workspace audit still fails for repositories without `.project.json`. This is intentional: registration alone is not adoption. For each remaining active project:

1. Run `scripts/init_project.py` with the correct profile and kind.
2. Replace the generated verification command with the repository's real command.
3. Review privacy, security, architecture, and deployment exceptions.
4. Run `scripts/workspace_check.py --project <id>`.
5. Run the project-local verification and CI.
6. Mark the registry entry `adoption: adopted` only after the evidence is recorded.

Learning, scratch, archived, and deployment-support directories should use the `inventory` or `deployment` profile instead of being forced into a product profile.

## Deployment readiness

A project is emitted into `deployment/projects.json` only when its
declaration sets `deployment.deployable: true`, names an existing
Compose file and Jenkins job, and passes the project checker (which
includes a matching `promote` entry in `deployment/promotions.json`).
Promotions are recorded exclusively through
`scripts/promote_project.py <id> --evidence-ref "<pointer>"`, and the
current manifest carries the seven pilot projects promoted with
`actor: workspace-governance/batch`; the next batch of promotions will
append new entries without overwriting prior ones. See
`docs/deployment-promotion.md` for the full transaction contract.

## Stale promotion triage (2026-09-26)

The seven promoted pilots (`alethefy`, `crossify`, `cvunify`, `hermexa`,
`hypora`, `lexora`, `openaccounting`) now report
`DEPLOYMENT_PROMOTION_STALE` because their recorded promotion revision
predates the current `HEAD`. Re-promotion was deliberately **not** performed:
recording a deployable decision at the current revision without a new release
would manufacture a promotion record rather than produce evidence. The
warnings are triage-only and name the remedy. Re-promote each project once its
release pipeline produces real artifacts:

```bash
python3 scripts/promote_project.py <id> --evidence-ref "<artefact pointer>"
```
