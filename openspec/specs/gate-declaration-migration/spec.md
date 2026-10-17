# gate-declaration-migration Specification

## Purpose
TBD - created by archiving change forge-gate-declaration-migration. Update Purpose after archive.
## Requirements
### Requirement: GDM-001 Preserve the Gate runtime's structured result and exit

Forge SHALL treat Driftwatchdog as the sole Gate policy/execution authority. For a current runtime document it SHALL map the row statuses `PASS`, `FAIL`, `REVIEW_REQUIRED`, `NOT_APPLICABLE`, and `UNBOUND` to five distinct Forge states; retain each row's structured `source`, `severity`, and bounded note; and retain the runtime's raw top-level status, blocked bit, exact numeric process exit code, and exact string lists `failures`, `pending_reviews`, `not_applicable`, and `unbound`. Forge SHALL keep `GateAggregate` only as its existing normalized compatibility projection and SHALL NOT reconstruct a runtime status, blocked decision, category list, or real-run exit from rows, declarations, attribution, diagnostics, or the compatibility aggregate. The real `forge gate` command SHALL return the captured runtime exit code.

#### Scenario: GDM-001-S1 Current row words remain distinct

- **WHEN** one structured runtime document contains rows with `PASS`, `FAIL`, `REVIEW_REQUIRED`, `NOT_APPLICABLE`, and `UNBOUND`
- **THEN** Forge persists and projects `pass`, `fail`, `review_required`, `not_applicable`, and `unbound` respectively, retains each row's exact structured source and severity, and counts none of the latter four as a pass

#### Scenario: GDM-001-S2 Unknown current word is unresolved

- **WHEN** a current runtime row contains a well-formed status outside those five words, including `SKIP`
- **THEN** Forge projects `unresolved`, preserves the raw word in the bounded row note, retains the runtime's top-level facts and exit unchanged, and does not produce a new `skip` state

#### Scenario: GDM-001-S3 Raw outcome survives a blocked run

- **WHEN** Driftwatchdog returns a parseable blocked document with non-empty categorized lists and exits `1`
- **THEN** Forge persists the raw top-level status, `blocked: true`, exit code `1`, and each list exactly, prints the same facts on every reader, persists the evidence, and `forge gate` exits `1` rather than deriving an exit from `GateAggregate`

#### Scenario: GDM-001-S4 Attribution cannot change runtime policy

- **WHEN** a runtime row passes but its declared implementation file or root is absent
- **THEN** only that row's implementation attribution becomes `absent`; its runtime state/source/severity, the top-level status, blocked bit, categorized lists, compatibility aggregate, and captured exit code remain unchanged

### Requirement: GDM-002 Validate an exact optional Forge verification schema

A schema-1 `forge.yaml` MAY omit `verification`; if the key is present it SHALL be a mapping with optional non-empty string `rule_pack` and required array `checks`. Forge SHALL preserve key presence before typed deserialization: explicit null, scalar, sequence, or mapping without `checks` is invalid rather than equivalent to omission. Each check SHALL contain unique kebab-case `id` and authored `source` exactly `native`, `bound`, `pack-default`, or `exempt`, and MAY contain `root` plus `path`. Root SHALL be exactly `project` or `governance-workspace`; root/path SHALL be both present or both absent; only `native` and `bound` MAY carry them. Paths SHALL be non-empty confined relative paths. Before any Gate dry/real spawn or read-path evidence short-circuit, `preflight_gate_declarations` SHALL reject duplicate/unknown/wrong-typed/invalid/unsafe/symlink-escaping Forge fields as `manifest-invalid` naming manifest and exact field. Invalid preflight SHALL leave invocation marker, prior evidence, operation journal, and sibling runtime history unchanged.

#### Scenario: GDM-002-S1 Verification is optional and checks may be empty

- **WHEN** Forge parses either a pre-change schema-1 manifest with no `verification` or `verification` containing only `checks: []`
- **THEN** both parse successfully, omission remains omission, and no rule-pack or check floor is invented

#### Scenario: GDM-002-S2 A complete filesystem-backed row is accepted

- **WHEN** a check has a unique valid id, `source: bound`, `root: governance-workspace`, and confined relative `path: workspace-governance/scripts/workspace_check.py`
- **THEN** Forge retains those exact typed values without resolving or writing a sibling during manifest parsing

#### Scenario: GDM-002-S3 Duplicate or unknown declaration is refused

- **WHEN** `verification` has a duplicate id, an unknown block/row field, an unknown source/root word, or a wrong-typed rule pack/check field
- **THEN** parsing fails with `manifest-invalid` naming `forge.yaml` and the exact later field such as `verification.checks[2].id`, and no Gate process or write begins

#### Scenario: GDM-002-S4 Invalid source and path combination is refused

- **WHEN** only one of root/path is present or a `pack-default`/`exempt` row carries root or path
- **THEN** parsing fails with `manifest-invalid` naming the offending `verification.checks[n].root` or `.path`

#### Scenario: GDM-002-S5 Unsafe path is refused before probing

- **WHEN** a path is empty, absolute, URL/drive-like, contains an empty/`.`/`..` component or backslash, or an existing candidate escapes its canonical declared root through a symlink
- **THEN** Forge returns `manifest-invalid` naming `verification.checks[n].path` and does not inspect or execute the escaped target

#### Scenario: GDM-002-S6 Present null or wrong container is not absent

- **WHEN** `verification` is explicitly null, a scalar, a sequence, or a mapping without `checks`
- **THEN** Forge returns `manifest-invalid` naming `verification` for null/scalar/sequence or `verification.checks` for the missing field; it does not accept an empty floor

#### Scenario: GDM-002-S7 Invalid verification has zero Gate side effects

- **WHEN** any Forge verification error is present before dry-run, real run, or status read, including when prior evidence exists
- **THEN** declaration preflight returns before Driftwatchdog spawn or evidence loading, the runtime invocation marker is absent, and prior evidence, operation journal, and sibling runtime history remain byte-identical

### Requirement: GDM-003 Attribute each check deterministically from declarations or structured runtime fields

For each newly captured runtime row Forge SHALL persist exactly one implementation source: `native`, `bound`, `pack-default`, `exempt`, `absent`, `unbound`, or `undeclared`. An exact case-sensitive floor id SHALL win. A declared `native`/`bound` row with a root/path SHALL retain its declared word only when the confined regular file exists; a missing root/file SHALL become `absent` and retain exactly one stable `<project>/...` or `<governance-workspace>/...` path. `governance-workspace` SHALL resolve only from the absolute `workspace_root` on a stored, enabled provider selection whose provider id is exactly `workspace-governance`. With no floor row Forge SHALL apply, in order, row `UNBOUND` → `unbound`, row `NOT_APPLICABLE` → `exempt`, exact runtime source `adapter:project-runtime` → `bound`, exact runtime source `driftwatchdog` → `native`, else `undeclared`. Forge SHALL NOT infer `pack-default` or inspect diagnostics, findings, evidence, commands, remediation, missing-evidence, or notes.

#### Scenario: GDM-003-S1 Exact floor row wins

- **WHEN** a runtime row's id exactly matches a floor entry whose authored source conflicts with its structured runtime source
- **THEN** Forge uses the floor entry and does not use a case-folded, prefix, or runtime-source match instead

#### Scenario: GDM-003-S2 Missing project implementation affects one row

- **WHEN** one declared `root: project` file is missing while other row files and runtime facts exist
- **THEN** only that row records `absent` and one `<project>/<relative-path>` value, while every other row and all runtime facts remain unchanged

#### Scenario: GDM-003-S3 Stored governance workspace resolves a file

- **WHEN** the stored provider selection is enabled, has id `workspace-governance`, carries an absolute `workspace_root`, and the declared file exists beneath that root
- **THEN** Forge uses the declared `native` or `bound` source and persists the stable `<governance-workspace>/<relative-path>` label rather than an absolute host path

#### Scenario: GDM-003-S4 Governance workspace is unavailable without fallback

- **WHEN** the selection/root/file is missing, the provider is disabled or has another id, or the declared candidate is not a regular file
- **THEN** only that row becomes `absent` with one stable path and Forge does not inspect an environment variable, adapter path, parent directory, or `../workspace-governance`

#### Scenario: GDM-003-S5 Runtime state provides observation fallback

- **WHEN** a row has no floor entry and reports `UNBOUND` or `NOT_APPLICABLE`
- **THEN** Forge attributes it `unbound` or `exempt` respectively before considering runtime source

#### Scenario: GDM-003-S6 Exact runtime source provides implementation fallback

- **WHEN** a row has no floor entry and is neither unbound nor not-applicable
- **THEN** exact `adapter:project-runtime` becomes `bound`, exact `driftwatchdog` becomes `native`, and any other structured source becomes `undeclared`; none becomes `pack-default`

#### Scenario: GDM-003-S7 Free text cannot influence attribution

- **WHEN** diagnostics, findings, commands, remediation, missing-evidence, or notes contain words such as `native`, `bound`, or `pack-default`
- **THEN** attribution is identical to the result with those free-text fields empty

### Requirement: GDM-004 Resolve current pack expectation and assess freshness without retroactive staleness

At read time Forge SHALL first preflight all current relevant declarations, then resolve pack expectation from the first non-empty identity in this order: `forge.yaml verification.rule_pack`; `.project.json verification.rule_pack`; the Driftwatchdog-selected `gate.toml`, `.driftwatch/gate.toml`, or `.ai-gate/gate.yaml`; otherwise undeclared. Preflight SHALL validate a present Forge manifest, the whole existing `.project.json`, the selected Gate manifest, and governance configuration only when a governance-root floor row requires it, even when a higher-precedence valid identity answers. Malformed Forge verification SHALL be `manifest-invalid`; malformed `.project.json` or selected Gate JSON/TOML/YAML SHALL be `gate-invalid` naming relative path and exact field; applicable malformed governance config SHALL remain `governance-invalid`. These errors SHALL precede evidence loading and absent-evidence projection. After valid preflight, the projection SHALL include identity/source/provenance and apply the existing revision/known-aggregate rule; declared mismatch/missing observed identity stales, while undeclared expectation leaves that rule unchanged and adds exactly `pack expectation undeclared`.

#### Scenario: GDM-004-S1 Pack precedence and provenance are deterministic

- **WHEN** multiple declaration locations carry different non-empty pack identities
- **THEN** Forge selects the first in the required precedence and reports its exact identity, source, and relative file provenance; removing it exposes the next source in order

#### Scenario: GDM-004-S2 Declared pack matches observed evidence

- **WHEN** revision-matched evidence with a known aggregate records the exact declared expected pack
- **THEN** pack assessment adds no stale reason and the existing revision/aggregate result remains fresh

#### Scenario: GDM-004-S3 Declared pack contradicts or is missing from evidence

- **WHEN** revision-matched evidence records another identity or no observed identity while a declaration exists
- **THEN** freshness is stale and the reason is `rule pack mismatch: expected <EXPECTED> from <SOURCE> (<PROVENANCE>), observed <OBSERVED>`, using `<unrecorded>` when absent

#### Scenario: GDM-004-S4 Pack expectation is undeclared

- **WHEN** no non-empty pack field exists in any of the three precedence tiers
- **THEN** revision-matched evidence with a known aggregate keeps its existing freshness, the projection adds `pack expectation undeclared`, and Forge makes no claim about which pack was intended

#### Scenario: GDM-004-S5 Malformed declaration does not fall through

- **WHEN** any existing relevant Forge, workspace, selected Gate, or applicable governance declaration is unreadable, malformed, wrong-typed, empty where present, or otherwise invalid
- **THEN** Forge returns `manifest-invalid`, `gate-invalid`, or `governance-invalid` as specified, naming path/field where applicable, and does not silently use another identity

#### Scenario: GDM-004-S6 Declaration error wins over absent evidence

- **WHEN** a CLI, doctor, checker, release, or API read targets a project with no persisted Gate evidence and a malformed current declaration
- **THEN** the read returns the typed declaration error before loading or short-circuiting on evidence and does not return the unverified projection

### Requirement: GDM-005 Project one Gate read model through every existing consumer

Forge SHALL build one Core `GateStatusProjection`/`GateEvidenceView` containing persisted runtime facts, per-row runtime state/source/severity/note, required projected implementation source and optional stable path, freshness reasons, and pack expectation. CLI JSON and human output, doctor, checker document, release preflight, admin API, and workbench SHALL consume that projection rather than independently loading, attributing, or assessing evidence. The checker SHALL carry the projection in an additive top-level `gate` field and SHALL NOT manufacture an alert for each passing Gate row. The workbench SHALL add text to existing Gate rows/facts and no new control.

#### Scenario: GDM-005-S1 Read surfaces agree

- **WHEN** the same persisted evidence and current project revision are read through `forge gate status` JSON/human, doctor, `forge check`, release preflight, and admin Gate status
- **THEN** every surface uses structurally equal row states, runtime source/severity, implementation source/path, raw runtime facts, freshness reasons, and pack expectation from one Core projection

#### Scenario: GDM-005-S2 Passing rows do not become checker alerts

- **WHEN** the Gate projection contains passing rows with implementation attribution
- **THEN** the checker document carries those rows under top-level `gate`, its alert array follows existing doctor finding rules, and no alert is synthesized merely because a row exists or passes

#### Scenario: GDM-005-S3 Workbench adds accessible text only

- **WHEN** the existing Gate card renders a projected row
- **THEN** the row text names id, state, runtime source, severity, implementation source, optional path, and note; freshness reasons are text; the existing re-run control remains the only Gate control; semantic list/fact structure, keyboard/focus behavior, no-motion behavior, and desktop/390-pixel no-overflow behavior remain intact

#### Scenario: GDM-005-S4 Valid declarations plus absent evidence stay unverified

- **WHEN** a managed project has no persisted Gate evidence and all current relevant declarations pass shared preflight
- **THEN** the projection reports `freshness: absent`, no gate record, and `the gate has never run; unverified is not a pass`; malformed declarations instead return their typed error and no surface fabricates rows, pack proof, or passing status

#### Scenario: GDM-005-S5 Release consumes rather than reruns

- **WHEN** release preflight evaluates Gate evidence
- **THEN** it uses the shared projection's freshness and compatibility aggregate, never invokes Gate, and refuses to cite revision-stale or declared-pack-mismatched evidence

### Requirement: GDM-006 Migrate additively and preserve generation/import semantics

The persisted Gate contract SHALL remain `0.1.0` and manifest schema SHALL remain `1`. Evidence written before this change SHALL remain readable and byte-identical on read; a missing stored implementation source SHALL project as `unrecorded` without inference, missing additive runtime facts SHALL remain absent/empty, and serialized old `state: "skip"` SHALL remain decodable only for compatibility. `forge new` SHALL emit exactly a multiline `verification` block with `checks: []`. Accepted import SHALL continue writing its existing minimal `forge.yaml` but SHALL omit `verification`.

#### Scenario: GDM-006-S1 Old evidence projects without rewrite

- **WHEN** Forge reads a valid pre-change `0.1.0` evidence file with no additive fields
- **THEN** every row projects implementation source `unrecorded`, optional raw fields remain absent, categorized lists are empty, no source is inferred from state/note, and file bytes remain unchanged

#### Scenario: GDM-006-S2 Legacy skip remains decode-only

- **WHEN** pre-change evidence contains serialized row state `skip`
- **THEN** Forge reads and displays that legacy state, while the current runtime parser has no branch that can produce it

#### Scenario: GDM-006-S3 New project gets an empty floor

- **WHEN** `forge new` generates a project manifest
- **THEN** its deterministic YAML contains exactly `verification:` followed by indented `checks: []`, with no invented rule pack or check

#### Scenario: GDM-006-S4 Accepted import keeps its minimal manifest

- **WHEN** a detected project is accepted through the existing import flow
- **THEN** Forge still creates the normal minimal schema-1 manifest and that manifest contains no `verification:` block

### Requirement: GDM-007 Keep no-manifest support at the direct Gate boundary

A direct Gate target MAY omit `forge.yaml` when `.project.json` or its directory supplies project identity and a selected Driftwatchdog Gate manifest supplies configuration. Absence SHALL be a valid empty floor; a present invalid Forge manifest SHALL be refused by shared preflight before run/status side effects. Forge SHALL NOT create `forge.yaml`, and this requirement SHALL NOT imply registration, doctor, checker orchestration, release, API, or workbench support for an unmanaged directory.

#### Scenario: GDM-007-S1 Cargo-shaped project runs directly

- **WHEN** an explicit directory contains `.project.json`, `.ai-gate/gate.yaml`, and a Gate runtime response but no `forge.yaml`
- **THEN** direct `forge gate <PATH>` and `forge gate status <PATH>` preserve and attribute every scheduled row, report the Gate-manifest pack provenance, and leave `forge.yaml` absent

#### Scenario: GDM-007-S2 Runtime fallback does not overclaim

- **WHEN** that project has no Forge floor and a runtime row uses an unknown structured source
- **THEN** the row is `undeclared`; Forge does not infer a binding from its command, diagnostic, findings, or project shape

#### Scenario: GDM-007-S3 Managed lifecycle is not broadened

- **WHEN** the direct no-manifest fixture passes
- **THEN** no registry row, provider selection, doctor/checker/release projection, admin route availability, or workbench project is created as a side effect or acceptance claim

### Requirement: GDM-008 Declare Forge's nine-check implementation floor

Forge's root schema-1 `forge.yaml` SHALL declare `verification.rule_pack: workspace-governance@1` and exactly these effective check sources: `repository`, `build`, `tests`, `security`, `declared-verification`, `governance-quality`, `placeholder-threshold`, and `product-code-boundary` as `bound`; `source-file-size` as `native`. Only filesystem-backed rows SHALL carry root/path: tests → `project` / `scripts/check-tests.sh`; declared-verification → `project` / `.project.json`; governance-quality → `governance-workspace` / `workspace-governance/scripts/workspace_check.py`; placeholder-threshold and product-code-boundary → `project` / `scripts/governance/product_code_quality.py`.

#### Scenario: GDM-008-S1 Root floor names all effective checks

- **WHEN** Forge parses its root manifest and compares the floor with the effective Driftwatchdog plan
- **THEN** the id sets are exactly equal to the nine required ids and every row has the required authored source

#### Scenario: GDM-008-S2 Only filesystem-backed rows name paths

- **WHEN** the floor is inspected
- **THEN** exactly tests, declared-verification, governance-quality, placeholder-threshold, and product-code-boundary carry the specified paired root/path, while repository, build, security, and source-file-size carry neither

#### Scenario: GDM-008-S3 Missing governance checkout is honest and isolated

- **WHEN** Forge has no enabled stored workspace-governance selection/root or the declared governance script is absent
- **THEN** governance-quality alone projects `absent` with `<governance-workspace>/workspace-governance/scripts/workspace_check.py`, and no parent/environment probe or runtime verdict change occurs

