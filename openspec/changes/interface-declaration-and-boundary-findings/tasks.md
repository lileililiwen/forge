# Tasks: Declared interfaces and boundary findings

## 1. BFS — Baseline and impact coverage

- [ ] Confirm the current declaration surface: every `properties` key and `$defs` entry in `schemas/project.schema.json`, and which keys the audit reads today.
- [ ] Confirm the audit's existing inputs and joins (registry + manifest + declaration) and enumerate the finding-code vocabulary it already declares, so the new codes follow the existing shape.
- [ ] Inventory the ports that reach across a boundary today: the generator command strings in `bind_gate_commands.py`, `bind_ui_gate.py`, `bind_verification_command.py`, and the vendored copies consumers keep (Forge's `scripts/governance/*`, the `secret_field_substrings` copies).
- [ ] Record the closed mechanism set and the declaration shape in design, including the two lawful alternatives to a sibling path.
- [ ] Confirm that no consumer reads an `interfaces` block today, so the addition cannot break a reader.
- **Done when** every changed surface, every finding code and every consumer is named.

## 2. DFS — Schema, audit and generator

- [ ] Extend `schemas/project.schema.json` with the optional `interfaces` array and its `$defs`, enforcing unique ids, exclusive `provides`/`consumes`, the closed mechanism set, the contract grammar, and the digest form.
- [ ] Add focused tests: each mechanism accepted, unknown mechanism rejected, version-less contract rejected, `sibling` without digest rejected, non-exclusive entry rejected, absent block silent.
- [ ] Implement the interface pass in `scripts/workspace_check.py`: validation, cross-project resolution, the seven interface findings with their exact codes and severities, and the remediation text naming the smallest lawful change.
- [ ] Implement `CAPABILITY_PROVIDER_UNVERIFIED` for registry entries whose provider declares no verified evidence, and confirm all nine current exemptions report it.
- [ ] Teach `bind_gate_commands.py` to write a matching `mechanism: sibling` declaration with a command digest whenever it binds a command reaching into another checkout, and to leave the concern unbound when it cannot describe the edge.
- [ ] Record the WARN→FAIL ramp date for `INTERFACE_SIBLING_PATH_UNPINNED` and `INTERFACE_VENDORED_WITHOUT_DIGEST` in `decisions.json` as a new decision.
- [ ] Document the declaration and the mechanism set in `docs/capabilities.md` (or the sibling document it points to).
- **Done when** a fixture project declaring each mechanism produces exactly the findings this change names, and no finding is emitted for an undeclared project.

## 3. BFS — Cross-surface regression and completeness

- [ ] Run the full Python suite, the workspace self-check and the auto-gate; the 82 existing declarations must report zero new findings.
- [ ] Dry-run the generators against a temporary scaffold: exactly one declared `sibling` edge next to the command, with the digest the audit recomputes; unresolved concerns stay absent.
- [ ] Verify the audit against a real repository that already carries a sibling binding (no declaration) and confirm the finding is a WARNING with a remediation, not an ERROR.
- [ ] Confirm the declaration is read by no other tool yet, and that removing the schema block restores the previous behaviour byte-for-byte.
- **Done when** every observation agrees with this change's scenarios and unrelated dirty files remain excluded.

## 4. Verification

- [ ] Run strict OpenSpec validation, the change-name check, and `git diff --check`.
- [ ] Record exact verification results in `HANDOFF.md`.
- [ ] Commit spec, code, tests and docs as commit 1; commit `HANDOFF.md` only as commit 2.

## Follow-up changes (other repositories, not this one)

- [ ] `forge` — `interface-declaration-consumption`: read declarations instead of the hardcoded builtin plugin table, and declare Forge's own interfaces in its `.project.json`.
- [ ] `platform-contracts` — an interface-descriptor envelope family, once its active-change queue is empty.
- [ ] `dotnet-platform-libs` — declare verified capability entries to unfreeze the nine exemptions, and publish a per-repository conformance entry point instead of scanning the whole workspace.
- [ ] `driftwatchdog` — publish a checker-protocol manifest and pin consumers to a release rather than `releases/latest`.
- [ ] per consumer — declare interfaces, then convert each `mechanism: sibling` edge to `consume` with a digest or to a published artifact.
