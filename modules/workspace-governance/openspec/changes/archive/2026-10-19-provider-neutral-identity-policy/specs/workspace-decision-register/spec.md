## ADDED Requirements

### Requirement: Identity-provider policy preserves vendor choice and population boundaries (REG-005)

The workspace decision register SHALL record shared workspace identity as provider-neutral standard OIDC policy, not a mandatory vendor choice. A changed accepted identity decision SHALL use the next permanent decision id and supersede the previous entry without reusing its id or deleting its history. The policy SHALL permit shared workspace-administrator authentication only for internal products, SHALL keep product end-user populations project-scoped by default, and SHALL retain the public static-site exclusion.

#### Scenario: Provider-neutral successor supersedes the prior identity decision

- **WHEN** a new accepted identity decision supersedes D2
- **THEN** D2 remains in the register with status `superseded`, the new permanent decision records standard OIDC and replaceable provider choice, and no single vendor is required by policy

#### Scenario: Shared workspace administrators remain distinct from product users

- **WHEN** the register records shared workspace administrator SSO and product end-user authentication
- **THEN** it states that application sessions and authorization remain local and that product user populations are isolated by default

#### Scenario: Public static-site exclusion remains binding

- **WHEN** the identity decision is audited
- **THEN** the public GitHub Pages catalog and sibling public static sites remain excluded from shared workspace login and cross-product identity data

#### Scenario: Policy declares no runtime migration

- **WHEN** the identity decision is audited
- **THEN** it names no migration or removal of the operator's present provider deployment, and leaves that change to a separately authorized runtime decision

#### Scenario: Consumer packages are linked to the policy

- **WHEN** the register lists the downstream packages that consume the policy
- **THEN** every referenced repository is a registered project id and every referenced change name is kebab-case

## MODIFIED Requirements

### Requirement: Decision lifecycle SHALL forbid id reuse and require supersede entries (REG-003)

Status values MUST be one of `proposed`, `accepted`, `rejected`,
`superseded`; changing an accepted decision requires a NEW entry whose
predecessor is marked `superseded`; ids are permanent. The register's first
content SHALL be the four toolchain decisions resolved on 2026-09-30
(D1–D4), and the D2 entry SHALL restate that the public GitHub Pages catalog
remains excluded from shared login.

#### Scenario: Supersede transition is representable

- **WHEN** a new entry with a fresh id supersedes an accepted one and the
  predecessor's status becomes `superseded` with its text retained
- **THEN** the register still loads with unique ids and no mutation of the
  old id

#### Scenario: Public-site exclusion is machine-checkable

- **WHEN** the seeded D2 entry is loaded
- **THEN** its `policy_boundary` field names the public-catalog exclusion
  (asserted by test string)

#### Scenario: Superseded entry keeps its historical text

- **WHEN** an accepted entry is marked `superseded` by a later entry
- **THEN** its id, title, date, decision text, policy boundary and downstream
  references are unchanged (asserted by test)

### Requirement: Checks and human view SHALL stay in sync (REG-004)

`workspace_check.py` SHALL expose a `--decisions` summary, the portfolio
audit SHALL emit exactly one absent-register warning (never per-project
errors), and `docs/toolchain-decisions.md` SHALL present every decision in
the register; `docs/capabilities.md` SHALL record that capability entry
schemas are owned by platform-contracts and registered instances by this
repository.

#### Scenario: Summary surfaces every entry

- **WHEN** `workspace_check.py --decisions --json` runs
- **THEN** output lists each decision's id, title, status, and date

#### Scenario: Portfolio audit tolerates adoption

- **WHEN** `workspace_check.py --root .. --audit` runs on the whole
  workspace with the register present
- **THEN** no per-project finding attributable to the register is raised

#### Scenario: Human view lists every register entry

- **WHEN** the human view document is compared with the register
- **THEN** the document table lists every register id and its stored status
  (asserted by test)
