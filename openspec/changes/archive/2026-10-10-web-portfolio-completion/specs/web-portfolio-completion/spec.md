# web-portfolio-completion (delta)

## ADDED Requirements

### Requirement: Complete portfolio read coverage in the browser

Forge SHALL serve per-project `tags`, `relations`, `reviews` and
`goals` lists plus the single-project `show` view through the
authenticated portfolio routes, reusing the existing typed registry
reads. Reads SHALL never probe a provider, run a native build, or
mutate state.

#### Scenario: List sub-resources read back what the writes stored

- **WHEN** an operator GETs `/v1/admin/portfolio/{id}/tags`,
  `/relations`, `/reviews` or `/goals` after recording metadata
- **THEN** each response carries the stored rows with provenance and
  the goals list contains only goals linked to that project

#### Scenario: Unknown read kind stays an honest 404

- **WHEN** an operator GETs `/v1/admin/portfolio/{id}/mystery`
- **THEN** Forge answers `404 portfolio-route-not-found` and changes
  nothing

### Requirement: Confirm-gated portfolio writes

Forge SHALL refuse every portfolio metadata write without an explicit
`confirm: true` with `409 portfolio-confirm-required`,
`effect: "none"`, and the current-state preview of the exact
sub-resource reviewed. No refused write SHALL change any row.

#### Scenario: Unconfirmed write changes nothing

- **WHEN** an operator posts a tag, relation, review, goal or evidence
  mutation without `confirm: true`
- **THEN** Forge answers 409 naming the missing confirmation,
  echoes the current preview, and the stored rows are identical
  before and after

#### Scenario: Confirmed writes apply and read back

- **WHEN** an operator posts each mutation with `confirm: true`
- **THEN** Forge persists it and the fleet list, show view and
  sub-resource list all render the new state

### Requirement: Tag and relation removal

Forge SHALL provide `POST /v1/admin/portfolio/{id}/tags/remove` and
`POST /v1/admin/portfolio/{id}/relations/remove`, each confirm-gated
and idempotent: removing an absent entry reports `removed: false`
with 200 and changes nothing.

#### Scenario: Remove round-trip

- **WHEN** an operator adds then removes a tag (or relation) with
  confirmation
- **THEN** the list contains it after the add and no longer after
  the remove; a second remove reports `removed: false`

### Requirement: Append-only evidence import from the browser

Forge SHALL provide `POST
/v1/admin/portfolio/{id}/evidence/import`, a confirm-gated
append-only snapshot import validated by the same Core vocabulary
the CLI uses. Editing or deleting imported evidence SHALL remain
refused: `POST /v1/admin/portfolio/{id}/evidence` keeps its `403
portfolio-source-owned` behavior with the snapshot provably
unchanged.

#### Scenario: Import appends, edits still refused

- **WHEN** an operator imports a valid snapshot with confirmation
- **THEN** it is stored and listed read-only with `editable: false`;
  a direct edit attempt is refused 403 and the stored count is
  unchanged; an import with an unknown status is refused 400 with
  nothing stored

### Requirement: Portfolio panel completes the metadata verbs

Forge SHALL render the show reader, tag remove, relation add/remove,
review history, goal add/link and evidence import/list controls in
the existing portfolio metadata card, each as preview → confirm →
apply with the card's established styles and accessibility patterns
(confirm checkboxes, `role="status"` previews, error-summary focus).
Imported evidence SHALL render read-only with no edit control.

#### Scenario: Operator completes every verb in the browser

- **WHEN** an operator drives each new control against a managed
  project with the confirmation ticked
- **THEN** every write applies and reads back, every validation
  failure surfaces through the error summary without state change,
  and the browser console reports zero errors
