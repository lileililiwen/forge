# deployable-batch-onboarding Specification

## Purpose

Make `deployable: true` an earned, recorded governance decision backed by
verification evidence, so the Jenkins manifest can be consumed
fail-closed downstream without freezing the portfolio.

## ADDED Requirements

### Requirement: Evidence-backed promotion transaction

A project MUST become deployable only through a promotion transaction that
requires a passing-audit state, an existing compose file, an explicit
evidence reference, and produces a ledger entry; the transaction MUST NOT
silently overwrite conflicting manual declarations.

#### Scenario: Clean promotion

- **WHEN** promotion runs for an adopted, checked-passing project with an
  evidence reference
- **THEN** its `deployment.*` block is updated, a ledger entry names the
  decision, actor, revision and evidence ref, and the manifest regenerates
  containing exactly those fields

#### Scenario: No evidence, no promotion

- **WHEN** promotion is requested without an evidence reference
- **THEN** nothing is modified and the refusal names the missing input

#### Scenario: Conflicting manual declaration protected

- **WHEN** the project already declares a different compose file than the
  detected one and no force flag is given
- **THEN** the transaction refuses and leaves all files unchanged

### Requirement: Ledger-authoritative deployable flag

The audit MUST report ERROR for any project whose declaration says
deployable while the promotions ledger holds no matching entry, so the
flag cannot be hand-edited into existence, and MUST warn (not block) when
the ledger entry's revision differs from the project's current HEAD.

#### Scenario: Hand-edited flag caught

- **WHEN** `.project.json` is edited to `deployable: true` outside the
  promotion transaction
- **THEN** the audit errors with `DEPLOYMENT_PROMOTION` and manifest
  generation refuses to emit the project

#### Scenario: Stale evidence triaged

- **WHEN** the promoted revision is older than the current HEAD
- **THEN** the audit passes with `DEPLOYMENT_PROMOTION_STALE` naming both
  revisions

### Requirement: Append-only promotion ledger

The ledger MUST reject removal or modification of existing entries on
load, MUST record demotions as entries rather than deletions, and MUST
round-trip promotions and demotions for the same project across history.

#### Scenario: Tampered ledger detected

- **WHEN** an existing ledger entry is edited out of the file
- **THEN** the checker fails loudly instead of treating the project as
  never promoted

### Requirement: First batch on the record

`deployment/projects.json` MUST contain exactly the projects with
completed promotion transactions; the pilot batch defined by this change's
evidence trail is the initial content, and every non-promoted project
MUST keep its previous non-deployable behavior.

#### Scenario: Empty-manifest freeze lifted

- **WHEN** jenkins-local consumes the regenerated manifest
- **THEN** promoted pilot projects resolve as eligible and an unpromoted
  product still resolves as refused
