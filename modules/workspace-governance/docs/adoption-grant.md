# Cross-repo adoption grant

An agent may repoint a consumer onto a registered shared provider, under a
bounded permission, without cross-repo authorization. This document is the
governance contract; the clause it backs lives in `templates/project/AGENTS.md`
and propagates through the shared rules baseline. See also
`docs/capabilities.md` for the adoption measurement that produces the finding
this grant lets an agent answer.

## Why the rule was amended

The shared agent rule used to say two contradictory things:

> Assess shared infrastructure before duplication. **Do not edit siblings
> without scope authorization.**

The first sentence invites reuse. The second makes adoption impossible,
because adopting a shared package into a product means writing a coordinate in
the product's own manifest and registering the shared type in the product's own
composition root — and deciding how the provider behaves is a provider change.
An agent that obeys both sentences concludes that hand-rolling a local
implementation is the only compliant outcome. That is a rational response to an
unsatisfiable rule, not a knowledge failure.

The amendment keeps the first sentence and makes the second precise. A
**source** change to another repository still needs scope authorization.
**Consuming** a sibling's published artifact does not.

## Grant shape

Declared in the **consumer's own** `.project.json`, so the authorization is
reviewed in the repository whose files it authorizes:

```json
"adoption_grant": {
  "capability": "idempotency",
  "provider": "dotnet-platform-libs",
  "package": "Platform.Idempotency",
  "version": "0.1.0",
  "actor": "agent:<session-id>",
  "issued_at": "2026-10-02T00:00:00Z",
  "expires_at": "2026-10-09T00:00:00Z",
  "write_set": [
    ".project.json",
    "Directory.Packages.props",
    "src/**/CompositionRoot.cs",
    "tests/**/*.cs"
  ],
  "rollback_ref": "openspec/changes/2026-10-05-adopt-idempotency",
  "status": "active"
}
```

`capability`, `provider`, `package`, `actor`, `issued_at`, `expires_at`, and
`write_set` are required. `expires_at` is required specifically so a grant
cannot silently become standing authorization; the duration itself is an owner
decision, the requirement that one exists is not. `version` is optional but,
when present, is the only version the grant authorizes.

Lifecycle: `issued → active → consumed | expired | revoked`. `consumed` is
terminal and **single-use** — a second repoint under one grant is
`ADOPTION_GRANT_STALE`, because the permission was sized for one change.

## What the grant permits

Every permitted write resolves inside the **consumer** root:

| Permitted | Why it is inside the boundary |
|---|---|
| `capabilities[].consumes[]` in the consumer's `.project.json` | The consumer's own declaration of what it consumes. |
| The consumer's package manifest (`Directory.Packages.props`, `*.csproj`, `package.json`, `Cargo.toml`, `pyproject.toml`) | A pinned coordinate is a consumer-side statement, not a provider change. |
| The consumer's composition root and call sites | Replacing a local helper with the shared one is the adoption itself. |
| The consumer's tests | Including the conformance check that proves no cross-repo reference exists. |
| The consumer's `AGENTS.md` | To record the adoption, or a `declines[]` boundary. |

## What the grant forbids

| Forbidden | Finding |
|---|---|
| Any write outside the consumer root, including through a symlink | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| Any write to the provider repository, at all | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| Bumping a provider version, or pinning a version the grant does not name | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| A `ProjectReference`, relative path, or `../` dependency on a sibling checkout | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| Promoting to `deployable: true` or writing the promotion ledger | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| Using an expired, revoked, or already-consumed grant | `ADOPTION_GRANT_STALE` |
| Consuming a provider that is not `verified` with a resolving evidence ref | `ADOPTION_GRANT_PROVIDER_UNVERIFIED` |
| A repoint with no adoption record, or a hand-edited record | `ADOPTION_GRANT_UNSATISFIED` |
| A malformed block, a missing field, or a `write_set` that escapes the root | `ADOPTION_GRANT_SHAPE` |

The version-bump prohibition is the sharpest edge. Adopting
`Platform.Idempotency` **0.1.0** is permitted; asking for 0.2.0 because the
consumer needs a method 0.1.0 lacks is not. That request is a provider change
and needs a provider change's authorization. A grant that could force a version
bump would be a grant to modify any provider in the workspace, wearing a
capability's name.

A `SCOPE_VIOLATION` is an error **regardless of the resulting code state**,
because the violation is the finding.

## The adoption record

`adoption/adoptions.json` is append-only and tamper-evident. One row per
repoint:

```json
{
  "kind": "adoption",
  "consumer": "some-product",
  "capability": "idempotency",
  "provider": "dotnet-platform-libs",
  "package": "Platform.Idempotency",
  "version": "0.1.0",
  "replaced": ["src/Infrastructure/IdempotencyGuard.cs"],
  "actor": "agent:<session-id>",
  "grant_expires_at": "2026-10-09T00:00:00Z",
  "rollback_ref": "openspec/changes/2026-10-05-adopt-idempotency",
  "recorded_at": "2026-10-05T12:00:00Z",
  "prev_hash": "",
  "content_hash": "<sha256 of the record body>"
}
```

Six things make an adoption auditable and none is optional: **what** was
replaced (`replaced`), **what** replaced it (`provider`/`package`/`version`,
pinned), **who** did it (`actor`), **under what authority**
(`grant_expires_at`), **how to undo it** (`rollback_ref`), and **when**
(`recorded_at`).

`content_hash` covers the record body; `prev_hash` chains to the row before it.
A hand-edited row fails its own hash, a hand-inserted row breaks the chain, and
both are refused with `ADOPTION_GRANT_UNSATISFIED` — the same posture as a
hand-edited `deployable: true` (`DEPLOYMENT_PROMOTION`).

A rollback is a **second** row with `kind: "rollback"` referencing the original.
It never edits or deletes the first, and it does not count as a second use of
the grant.

## What the check does and does not do

`validate_adoption_grant()` is read-only, like every other audit check. It
proves an authorization existed, was in scope, and produced a recorded,
unmodified outcome. It does not inspect build output, run a test, or verify
that the repoint is *correct* — that is the consumer's own gate. This is the
same boundary the rest of the repository holds: **governance records, product
gates verify.**

`ADOPTION_GRANT_SCOPE_VIOLATION` is an ERROR, so the unattended gate
(`scripts/auto_gate.py`) already fails on it through the existing
`summary["errors"]` rule. No change to the gate or to the report schema was
required.

## Boundaries this grant does not touch

- `projects.json` is unchanged: no `id`, `path`, `adoption`, or `lifecycle`
  value moves.
- `deployment/promotions.json` is not extended, reused, or modified. Adoption
  is not promotion and is never a path to `deployable: true`.
- No provider repository is edited, published, version-bumped, or retargeted.
- No ambient source auto-adoption, no cross-repository `ProjectReference`, and
  no absolute sibling paths. Those stay forbidden by `shared-ownership.md` and
  this grant does not relax them.
- The first adoption executed under a grant is a separate, individually
  reviewed change with its own rollback.

## Ownership

| Surface | Owner |
|---|---|
| Grant declaration and its review | The consumer repository, in its own `.project.json` |
| Adoption ledger | Workspace Governance — `adoption/adoptions.json` |
| Grant checks | `scripts/workspace_check.py`, read-only |
| Provider behaviour and version | The provider repository's own change process |
