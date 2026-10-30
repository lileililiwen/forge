# Change: gate-declaration-reconciliation

## Why

Forge's own Gate declarations disagree with each other and with the promoted
specs. `forge.yaml` still declares a `governance-quality` check whose path is
`forge/modules/workspace-governance/scripts/workspace_check.py`, and a rule pack
of `workspace-governance@1`. The effective plan has neither: it runs eight
checks under `forge-baseline/1.0.0` and reports `unbound required checks: none`.

The promoted spec `gate-declaration-migration` GDM-008 says Forge's floor is
*nine* checks with exact set equality against the plan. `forge-native-gate`
says `governance-quality` is an explicit opt-in, never implicit. Both are
canonical, and the live configuration satisfies neither. The dry-run cannot see
the dropped row, so a declared check that never runs is reported as a clean
plan — the exact "a verifier that no gate runs does not gate" failure the
declaration model exists to prevent.

The regression test that should catch this parses a recorded fixture from
September: nine checks, `workspace-governance@1`, all timeouts 60 s. It passes
while the live plan differs, because nothing compares the fixture or the
declaration to the plan Forge actually resolves.

## What Changes

- **`forge.yaml` declares the effective floor.** The stale `governance-quality`
  row is removed, `verification.rule_pack` becomes `forge-baseline/1.0.0`, and
  the remaining path rows are the ones that exist.
- **A live parity contract test compares the declaration to the plan.** It
  reads `forge.yaml` and runs `forge gate --dry-run --format json`, then
  asserts exact id-set equality, the same rule pack, every declared path
  present, and no unbound required check. A declared row that the plan does not
  select fails by name.
- **The rehearsal fixture is regenerated from the live runtime.** It becomes an
  eight-check `forge-baseline/1.0.0` capture, and its assertions stop claiming
  all timeouts are 60 s.
- **The spec is reconciled.** GDM-008's nine-check requirement is removed and
  replaced by an effective-floor requirement that matches `forge-native-gate`:
  `governance-quality` is opt-in, and the floor equals the plan.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `forge.yaml` | check floor and rule pack |
| `tests/gate_declaration_parity.rs` | new live parity contract |
| `tests/fixtures/gate/gate-rehearsal-plan.json` | regenerated live capture |
| `src/gate/rehearsal_tests.rs` | assertions follow the capture |
| `openspec/specs/gate-declaration-migration/spec.md` | GDM-008 removed and replaced |
| `.ai-gate/gate.yaml` | unchanged; it is the runtime selection the floor must match |

Persistence: none. No Gate evidence shape changes.

Failure boundaries: a declared check absent from the plan fails the parity test;
a plan check absent from the declaration fails it too; a declared path that
does not resolve fails it; a rule-pack mismatch fails it.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `gate-declaration-migration` | modified | GDM-008 reconciled with `forge-native-gate` |
| `forge-native-gate` | read only | the governing model |
| `gate-runtime-evidence` | read only | evidence shape unchanged |

## Non-Goals

- No change to the Gate engine, the baseline pack, or the runtime plan. The
  declaration is aligned to the plan, not the plan to the declaration.
- No restoration of `governance-quality` as an implicit check. A project that
  wants it selects it explicitly.
- No signing, publishing or release work.
