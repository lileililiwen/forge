# cross-repo-adoption-grant Specification

## Purpose
Allow a consumer project to adopt one registered shared capability under a
bounded, single-use permission, with checks that prevent the permission from
reaching provider source, version, deployment, or promotion state.
## Requirements
### Requirement: Grant a bounded cross-repo adoption permission

Workspace Governance SHALL allow a project to declare an `adoption_grant`
that permits one agent session to repoint that project onto one registered
shared provider for one capability, without separate cross-repo
authorization. A grant SHALL name exactly one consumer project, one
capability, one provider, one package, one actor, an issue time, a mandatory
expiry, and a bounded `write_set`. A grant SHALL be single-use.

#### Scenario: Grant is declared and bounded

- **WHEN** a project's `.project.json` carries an `adoption_grant` naming a
  registered provider, a capability already bound to that provider, an
  actor, `issued_at`, `expires_at`, and a `write_set`
- **THEN** the audit records the grant as active and emits no
  `ADOPTION_GRANT_*` finding

#### Scenario: Grant may not create a binding

- **WHEN** an `adoption_grant` names a provider that is not a registered
  project id, or names a capability that is not already bound to that
  provider in the capability registry
- **THEN** the audit refuses the grant with a finding naming the
  unresolved identity and validates no write under it

#### Scenario: Malformed grant block

- **WHEN** the `adoption_grant` block is not an object, is missing any of
  `capability`, `provider`, `package`, `actor`, `issued_at`, `expires_at`, or
  `write_set`, or carries a `write_set` pattern containing an absolute path
  or a `..` segment
- **THEN** the audit emits `ADOPTION_GRANT_SHAPE` naming the offending field
  and does not validate any write under the grant

### Requirement: Confine the grant to consumer-side writes

A grant SHALL permit only writes that resolve inside the consumer project
root: the consumer's `capabilities[].consumes[]` declaration, the consumer's
own package manifest, the consumer's own composition root and call sites,
the consumer's own tests, and the consumer's own `AGENTS.md`. A grant SHALL
NOT permit any write to the provider repository, a provider version bump, a
source-level coupling to a sibling checkout, a promotion to `deployable`, or
a write to the promotion ledger. Consuming a published pinned artifact SHALL
be treated as a consumer-side change; creating a source-level coupling SHALL
NOT.

#### Scenario: Permitted consumer-side repoint

- **WHEN** a repoint under an active grant edits only the consumer's own
  manifest to add a pinned coordinate, registers the shared type in the
  consumer's own composition root, and updates the consumer's own tests
- **THEN** the audit accepts the repoint and emits no
  `ADOPTION_GRANT_SCOPE_VIOLATION`

#### Scenario: Write into the provider repository

- **WHEN** any write under a grant resolves inside the provider project root
- **THEN** the audit emits `ADOPTION_GRANT_SCOPE_VIOLATION` as an error and
  does not read or write the file, regardless of the resulting code state

#### Scenario: Provider version bump

- **WHEN** a repoint changes a provider's version, or edits a provider
  manifest, or requests a provider version other than the one the grant
  names
- **THEN** the audit emits `ADOPTION_GRANT_SCOPE_VIOLATION` as an error
  naming the provider, because a version change is a provider change and
  requires that change's own authorization

#### Scenario: Source-level coupling to a sibling checkout

- **WHEN** a repoint adds a `ProjectReference`, a relative path, or a `../`
  dependency pointing at a sibling repository checkout
- **THEN** the audit emits `ADOPTION_GRANT_SCOPE_VIOLATION` as an error,
  and the prohibition holds regardless of any active grant

#### Scenario: Write outside the consumer root

- **WHEN** a path matched by `write_set` resolves outside the consumer
  project root, including through a symlink or a manifest indirection
- **THEN** the audit emits `ADOPTION_GRANT_SCOPE_VIOLATION` and does not
  read or write the path

#### Scenario: Promotion is never in scope

- **WHEN** a repoint sets `deployable: true`, writes
  `deployment/promotions.json`, or produces a Jenkins manifest entry
- **THEN** the audit emits `ADOPTION_GRANT_SCOPE_VIOLATION` as an error,
  because adoption is not promotion and is never a path to `deployable`

### Requirement: Expire and retire a grant

A grant SHALL be bounded in time and in use. The audit SHALL report an
expired, revoked, or already-consumed grant as `ADOPTION_GRANT_STALE` for
any further repoint, and SHALL NOT retroactively legalize a repoint that was
executed outside a grant's window.

#### Scenario: Expired grant

- **WHEN** `expires_at` is in the past at audit time and a repoint is
  present or declared
- **THEN** the audit emits `ADOPTION_GRANT_STALE` naming the expiry and does
  not treat the repoint as authorized

#### Scenario: Grant is single-use

- **WHEN** a grant that has already been consumed is used to authorize a
  second repoint
- **THEN** the audit emits `ADOPTION_GRANT_STALE` and refuses the second
  repoint, because the permission was sized for one change

#### Scenario: Revoked grant

- **WHEN** a grant is marked revoked
- **THEN** the audit emits `ADOPTION_GRANT_STALE` for any repoint made under
  it after revocation

### Requirement: Require a verified provider

The audit SHALL require the provider's declared capability to be `verified`
with a resolving `evidence_ref` before a grant may authorize a repoint. The
audit SHALL NOT allow a grant to substitute for the provider's own evidence.

#### Scenario: Provider not verified

- **WHEN** a grant names a provider whose capability is `declared`,
  `configured`, or `unverified`, or whose `evidence_ref` does not resolve
- **THEN** the audit emits `ADOPTION_GRANT_PROVIDER_UNVERIFIED` naming the
  provider and its declared state, and the repoint is not authorized

### Requirement: Record every granted adoption

Every repoint executed under a grant SHALL produce an immutable adoption
record in `adoption/adoptions.json` carrying the replaced local paths, the
pinned provider coordinates, the actor, the grant expiry that authorized it,
a rollback reference, and the record time. The ledger SHALL be append-only.
A rollback SHALL be recorded as a second entry and SHALL NOT edit or delete
the original.

#### Scenario: Adoption recorded

- **WHEN** a repoint is executed under an active grant
- **THEN** an adoption record is appended carrying the replaced local
  paths, the pinned `provider`, `package`, and `version`, the `actor`, the
  authorizing `grant_expires_at`, the `rollback_ref`, and the
  `recorded_at`

#### Scenario: Repoint with no record

- **WHEN** a consumer declares `capabilities[].consumes[]` for a granted
  capability and no matching adoption record exists in
  `adoption/adoptions.json`
- **THEN** the audit emits `ADOPTION_GRANT_UNSATISFIED` naming the consumer
  and capability, and the adoption is not treated as authorized

#### Scenario: Hand-edited record

- **WHEN** a record's body does not match the `content_hash` recorded for it
- **THEN** the audit emits `ADOPTION_GRANT_UNSATISFIED` and treats the
  record as untrusted, mirroring the refusal of a hand-edited
  `deployable: true`

#### Scenario: Rollback preserves the original

- **WHEN** a granted adoption is rolled back
- **THEN** a second entry with `kind: "rollback"` referencing the original
  is appended, and the original record is unchanged and still present

### Requirement: Amend the agent rule so adoption is lawful

The shared `AGENTS.md` clause SHALL be amended so that it invites reuse and
permits it. The clause SHALL require an agent to name the specific sibling,
project id, or package it assessed, SHALL continue to forbid source-level
changes to a sibling repository without scope authorization, and SHALL
explicitly permit repointing a consumer onto a registered shared provider
under a valid `adoption_grant`. The amendment SHALL NOT relax the
prohibition on source-level coupling, provider version bumps, ambient
auto-adoption, or promotion.

#### Scenario: Assessment must name its target

- **WHEN** the amended clause directs an agent to assess shared
  infrastructure
- **THEN** the clause requires the agent to name the specific sibling,
  project id, or package it assessed, and treats an un-named assessment as
  not an assessment

#### Scenario: Adoption is now permitted under a grant

- **WHEN** an agent holds a valid, unexpired, unconsumed `adoption_grant`
  in the consumer's `.project.json`
- **THEN** repointing the consumer onto the registered provider is
  authorized without separate cross-repo approval, and the agent records
  the adoption in the consumer's `capabilities[].consumes[]` and in
  `adoption/adoptions.json`

#### Scenario: Source edits remain forbidden

- **WHEN** an agent without a grant proposes to change a sibling
  repository's source
- **THEN** the amended clause still requires scope authorization, exactly
  as before the amendment

#### Scenario: Legitimate boundary arguments are preserved

- **WHEN** a repository documents that ownership, privacy, lifecycle, or
  dependency direction makes the capability correctly local
- **THEN** the amended clause still permits keeping the capability local,
  and the boundary argument remains a valid reason not to adopt

### Requirement: Declare stable finding codes

The audit SHALL report grant state under the stable identifiers
`ADOPTION_GRANT_SCOPE_VIOLATION`, `ADOPTION_GRANT_UNSATISFIED`,
`ADOPTION_GRANT_STALE`, `ADOPTION_GRANT_PROVIDER_UNVERIFIED`, and
`ADOPTION_GRANT_SHAPE`. These codes SHALL be additive: no existing finding
code, severity vocabulary, exit code, report key, promotion-ledger value, or
Forge adapter observation key SHALL be renamed, removed, or re-leveled.

#### Scenario: Existing findings are unchanged

- **WHEN** a project with no `adoption_grant` block is audited
- **THEN** the finding set contains every finding it contained before and no
  `ADOPTION_GRANT_*` finding, and the JSON report, promotion ledger, and
  Forge adapter observation keys are structurally identical to the previous
  version
