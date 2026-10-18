# Design: Classify dirty-manifest debt without writing target files

## Implementation boundary

Modify only `workspace-governance/scripts/close_gate_debt.py`, its unit tests, OpenSpec artifacts, generated `gate-debt.json`, and the governance `HANDOFF.md`. Keep sibling repositories unchanged. The existing `gate-debt.json` input and JSON report envelope remain compatible.

## Language and runtime

Python 3.11+ standard library. Verify with `python3 -m unittest tests.test_close_gate_debt -v`, the full `python3 -m unittest discover -s tests -v` suite, and `openspec validate --changes --strict`.

## Ownership and shared code

Workspace Governance owns debt classification and operator guidance. `bind_gate_commands.py` remains the source of the dirty-manifest reason and sole writer of generated gate bindings. Product manifests remain owned by their repositories; the driver must not write them.

## Behavioral model

| Input reason | Class | Driver behavior |
|---|---|---|
| Contains the binder's exact dirty-manifest refusal marker | G | Report the affected repo/check and ask its owner to review/commit the manifest, then rerun the binder; never dispatch a remedy |
| Matches no known reason | ? | Preserve unknown escalation behavior |
| Matches existing A-F reason | Existing class | Preserve current behavior |

Class G is a reporting class, not a repair class. It is counted as a decision in the existing report totals. Dry and apply modes produce the same class G guidance and do not change target files.

## Contract and compatibility

No input-schema or JSON shape changes. The existing `class` field carries `G`; `question` contains the operator action. Existing consumers that display arbitrary class strings continue to work. `APPLY` remains restricted to A-D.

## Failure and boundary behavior

- A changed or unfamiliar reason that does not contain the exact refusal marker remains `?`.
- Dirty manifests remain untouched in dry and apply modes.
- Missing/corrupt debt records retain their current fail-closed behavior.
- A successful binder rerun is required to clear the recorded debt; class G itself never claims the gate check is bound or passing.

## Verification oracle

Unit tests cover exact reason recognition, unknown near-matches, decision count and message, and a fixture showing class G does not call any remedy or mutate the target manifest. Run the full suite and strict OpenSpec validation. Run the driver against the regenerated full-workspace debt record and verify no dirty-manifest entries are class `?` and no changes occur in sibling manifests.

## Decision ledger

- Use class G because A-F already have stable meanings and the dirty-manifest state is known but requires its repository owner.
- Use the binder's exact refusal marker; do not infer dirty state from Git or inspect/modify the product manifest in the driver.
- Include the generated canonical debt snapshot because it is the direct output of the already authorized fleet binder application and supplies the real integration fixture.
