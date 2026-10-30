## REMOVED Requirements

### Requirement: GDM-008 Declare Forge's nine-check implementation floor
- **Driver:** `gate-declaration-reconciliation`
- **Source spec:** `openspec/specs/gate-declaration-migration/spec.md`
- **Requirement removed:** the nine-check floor, its
  `verification.rule_pack: workspace-governance@1` clause, and the
  `governance-quality` path row. `forge-native-gate` made the bundled
  `forge-baseline/1.0.0` authoritative and made `governance-quality` an
  explicit opt-in; the nine-check floor contradicts both.
- **Verification oracle:** the floor now equals the effective plan, proven by
  `tests/gate_declaration_parity.rs`, and the dry-run reports eight checks with
  no unbound required check.

## ADDED Requirements

### Requirement: GDM-008 Declare Forge's effective check floor

Forge's root schema-1 `forge.yaml` SHALL declare
`verification.rule_pack: forge-baseline/1.0.0` and exactly the check sources the
runtime plan selects: `repository`, `build`, `tests`, `security`,
`declared-verification`, `placeholder-threshold` and `product-code-boundary` as
`bound`; `source-file-size` as `native`. Only filesystem-backed rows SHALL
carry a `root`/`path` pair: tests → `project` / `scripts/check-tests.sh`;
declared-verification → `project` / `.project.json`; placeholder-threshold and
product-code-boundary → `project` / `scripts/governance/product_code_quality.py`.
`governance-quality` SHALL NOT be declared unless the runtime manifest also
selects it.

#### Scenario: GDM-008-S1 Floor equals the plan

- **WHEN** Forge parses its root manifest and compares the declared ids with
  the ids in `forge gate --dry-run --format json`
- **THEN** the sets are exactly equal and no declared row is missing from the
  plan

#### Scenario: GDM-008-S2 Only filesystem-backed rows name paths

- **WHEN** the floor is inspected
- **THEN** exactly tests, declared-verification, placeholder-threshold and
  product-code-boundary carry the specified paired root/path, while repository,
  build, security and source-file-size carry neither

#### Scenario: GDM-008-S3 Governance quality is opt-in

- **WHEN** a project does not select `governance-quality` in its runtime
  manifest
- **THEN** the plan contains no governance-quality row and the floor does not
  declare one

#### Scenario: GDM-008-S4 A declared row the plan does not run is refused

- **WHEN** a floor row names a check the runtime plan does not select, or the
  plan selects a check the floor does not declare
- **THEN** the parity contract test fails naming the row and the plan, rather
  than reporting an empty unbound list
