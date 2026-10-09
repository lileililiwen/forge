# web-lifecycle-execution Specification

## Purpose
TBD - created by archiving change web-lifecycle-execution. Update Purpose after archive.
## Requirements
### Requirement: Graduation preview/import runs in the workbench

The workbench idea entry SHALL execute graduation preview/import through typed in-process admin routes with digest-bound confirmation, running the same `parse_artifact`/`validate_graduation`/`build_proposal`/`adopt_graduation` Core path as the CLI with no shell and no browser-supplied path.

#### Scenario: Preview validates without writing

- **WHEN** the operator pastes artifact JSON, picks a profile, optionally overrides the id, and presses Preview
- **THEN** `POST /v1/admin/graduation/preview` returns the brief title, requirements/evidence counts, source contract/revision, and proposed id plus a `plan_digest` with `effect:none`, writing nothing to disk, registry, or journal.

#### Scenario: Confirm imports the same bytes

- **WHEN** the operator ticks the confirm box and presses Run on the reviewed preview
- **THEN** `POST /v1/admin/graduation/import` with `confirm:true` and the exact `plan_digest` creates the project under the server-configured root, writes the receipt, registers, records journal kind `graduation.import`, and announces via a `role=status` result plus a journal evidence row.

#### Scenario: Stale preview is refused

- **WHEN** the confirm carries a missing or changed digest
- **THEN** the server refuses (400/409) with a fresh preview plus fresh digest, writes nothing, moves focus to the error summary, and the card re-arms on the fresh digest.

### Requirement: Intent resolve/apply runs in the workbench

Per-project intent SHALL resolve to a reviewable plan and apply only on digest-bound confirmation through the same `validate_intent`/`resolve_plan`/`write_plan_receipt`/`apply_plan` Core path as the CLI, with stale plans refused and journal evidence recorded.

#### Scenario: Resolve previews without writing a receipt

- **WHEN** the operator fills typed intent fields and presses Preview
- **THEN** `POST /v1/admin/projects/{id}/intent/resolve` returns the plan (`plan_id`, steps, `intent_hash`, `catalog_hash`) plus `plan_digest` with `effect:none`, writing no receipt, registry row, or journal entry.

#### Scenario: Confirm applies the reviewed plan

- **WHEN** the operator ticks confirm and presses Run with the exact digest
- **THEN** `POST .../intent/apply` writes the receipt, applies the plan in-process, records journal kind `intent.apply`, and announces via `role=status` plus a journal evidence row.

#### Scenario: Stale intent plan is refused

- **WHEN** the digest is missing, forged, or the catalog/profile moved under the reviewed plan
- **THEN** the server refuses 409 with the recomputed plan plus fresh digest, writes nothing, focuses the error summary, and the card re-arms.

### Requirement: Remediate plan/apply runs in the workbench

Per-project remediation SHALL plan from a finding id and apply only on digest-bound confirmation through the same `build_plan`/`apply(confirm=true)` Core path as the CLI with the target resolved server-side; the terminal `forge remediate plan --finding <id>` string stays where shown.

#### Scenario: Plan previews without writing

- **WHEN** the operator enters a finding id and presses Preview
- **THEN** `POST /v1/admin/projects/{id}/remediate/plan` returns the plan (`plan_id`, actions) plus `plan_digest` with `effect:none`, writing nothing.

#### Scenario: Confirm applies the reviewed plan

- **WHEN** the operator ticks confirm and presses Run with the exact digest
- **THEN** `POST .../remediate/apply` applies in-process with `confirm=true`, records journal kind `remediate.apply`, and announces via `role=status` plus a journal evidence row.

#### Scenario: Stale remediate plan is refused

- **WHEN** the digest is missing or the finding set changed under review
- **THEN** the server refuses 409 with the fresh plan plus fresh digest, writes nothing, focuses the error summary, and the card re-arms.

### Requirement: Every confirm is clickable end-to-end with journal evidence

Every lifecycle confirm (idea, scaffold `new`, spec save, refine, gate dry-run plan display, delivery approve/publish, maintain refresh, remediate plan/apply, intent resolve/apply, next-idea) SHALL follow preview → confirm-checkbox → run → `role=status` result plus a journal evidence row, with no dead buttons.

#### Scenario: Confirm without preview is refused

- **WHEN** Run is pressed with no preview digest or an unticked confirm box
- **THEN** nothing is written, the card shows the refusal, and focus lands on the error summary.

#### Scenario: Success records journal evidence

- **WHEN** a confirmed run succeeds
- **THEN** the project journal gains the operation row and the workbench operations table re-renders it.

### Requirement: Delivery next-idea transition journals the loop

Delivery publish success SHALL record a `delivery.next-idea` journal row and render it in `#delivery-next-idea` with the `&step=idea` link plus the maintain refresh shortcut.

#### Scenario: Publish lands the next idea

- **WHEN** `POST .../publish` (or delivery publish) succeeds
- **THEN** the server records kind `delivery.next-idea` and `#delivery-next-idea` announces it via `role=status` naming the journal operation.

### Requirement: Studio refine confirm shows the revision bump

Studio refine SHALL reuse `POST /v1/admin/projects/{id}/studio/refine` with confirmation and display the returned `spec_revision`/`app_revision` bump in a `role=status` result plus the journal evidence row.

#### Scenario: Refine bumps the revision

- **WHEN** the operator confirms a refine request
- **THEN** the card shows the old → new revision pair and the operations table shows the `studio.refine` row.

