## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| `forge.yaml` project floor | `forge` | Yes |
| `.ai-gate/gate.yaml` runtime selection | `forge` | No — it is the plan the floor must match |
| The live parity contract | `forge` | Yes |
| The rehearsal fixture and its parser test | `forge` | Yes, capture refresh |
| The Gate engine and baseline | `forge` | No |

## The Two Specs, Reconciled

`gate-declaration-migration` GDM-008 was written when the runtime was
`workspace-governance@1` and the floor was nine checks. `forge-native-gate`
later made the bundled Forge baseline authoritative and made
`governance-quality` an explicit opt-in. The newer requirement wins; GDM-008 is
removed and replaced, not silently edited, so the archive records the change of
intent.

## Why Remove the Row Instead of Restoring the Check

`forge.yaml` selected `governance-quality` with a root of `governance-workspace`
and a path of `forge/modules/workspace-governance/scripts/workspace_check.py`.
The runtime manifest (`.ai-gate/gate.yaml`) does not select the concern and
binds no command for it. Two honest repairs exist:

1. add the concern and its command to `.ai-gate/gate.yaml`, so the declared
   check runs; or
2. remove the declaration, so the floor matches the plan.

`forge-native-gate` answers which is correct: a project selects
`governance-quality` explicitly. Forge's own project did not do so in the
runtime manifest, and the shared governance checker is not part of the bundled
baseline. The declaration was the stale half. Removing it keeps the Gate
narrower than the declaration, which is the safe direction; adding a check
Forge does not need would widen every future run.

The evidence that the row was dead: the plan has eight checks, the recorded
evidence has eight, and the declared path does not exist at the stated root.

## The Parity Contract

`tests/gate_declaration_parity.rs` is the guard the old fixture was not.

```text
read forge.yaml
  -> declared ids, rule pack, path rows
run forge gate --dry-run --format json
  -> planned ids, rule pack, unbound required checks
assert declared ids == planned ids
assert declared rule pack == planned rule pack
assert every project-rooted path is a file
assert unbound_required_checks is empty
```

It runs the built binary from the repository root, so it observes the same
`.ai-gate/gate.yaml` resolution an operator does. It executes no check and
writes no evidence (dry-run).

The failure it prevents is one-directional in practice but checked both ways:
a stale declared row (today's bug) and an undeclared plan row both fail.

## The Fixture

`tests/fixtures/gate/gate-rehearsal-plan.json` is regenerated from
`driftwatchdog gate --dry-run --format json` on this repository. The parser
test keeps its job — proving a real runtime document parses into the typed
structure — but its assertions follow the new capture: eight checks, two
600 000 ms test checks, `forge-baseline/1.0.0`, and digest
`sha256:2c456f0e82a570fb`.

## Failure Boundaries

- A future row added to `.ai-gate/gate.yaml` without a floor declaration fails
  the parity test naming the plan-only id.
- A future floor row removed from the runtime manifest fails naming the
  declaration-only id.
- A broken path fails naming the row and the relative path.
- The Gate itself is unchanged, so a passing parity test cannot make a blocked
  check pass.

## Verification Strategy

1. `cargo test --test gate_declaration_parity` passes.
2. `cargo test -p forge --lib gate::` passes with the regenerated fixture.
3. `forge gate --dry-run --format json` reports eight checks and
   `forge-baseline/1.0.0`.
4. The repository suite, formatting, clippy, OpenSpec validation and the full
   Gate pass.
