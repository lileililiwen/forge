# Design: Grant bounded cross-repo capability adoption

## 1. Implementation boundary

Repository `/home/paul/code/workspace-governance`, Python 3 standard library
only. No new dependency.

- **Extension points are exactly:** `scripts/workspace_check.py` (one new
  `validate_adoption_grant()` plus four new `ADOPTION_GRANT_*` codes, called
  from `validate_project()`), the new governance-root ledger directory
  `adoption/` holding `adoptions.json`, and the shared `AGENTS.md` rule
  clause, which is amended as text. `scripts/auto_gate.py` is **not**
  modified — `ADOPTION_GRANT_SCOPE_VIOLATION` is an ERROR, so the existing
  `summary["errors"]` rule already fails the unattended gate.
- **No change to** `projects.json`, `vocabulary.json`, `decisions.json`,
  `deployment/promotions.json`, any existing finding code, any exit code, or
  the Forge adapter's observation keys.
- **No product or platform repository is modified by this change.** The
  `AGENTS.md` amendment is delivered as a clause the shared-rules baseline
  (`rules-baseline`, the single-source change that already exists in this
  repo) propagates; individual repos receive it through that mechanism, not
  by a change here.

## 2. The ruling

**An agent may repoint a consumer onto a registered shared provider, under a
bounded permission, without cross-repo authorization.**

Three words carry the ruling and each is load-bearing:

- **registered** — the provider must be a known project id and the capability
  must already be bound. The grant cannot invent a provider.
- **bounded** — one consumer, one capability, one provider, one named file
  set, one expiry, single use. The bound is what makes it safe to skip the
  authorization that normally guards a cross-repo edit.
- **consumer** — the write set is one root. The provider is read-only, always.

The grant removes exactly one obstacle — "Do not edit siblings without scope
authorization" — and creates a larger permission to do the same thing
badly. That is the intended trade: a constrained, recorded, single-use
repoint is strictly safer than an unconstrained local reimplementation that
nobody can see.

## 3. Why an edit at all

The subtle point: adopting a provider *looks* like a local edit but is not
one. The consumer adds a pinned coordinate to its own manifest and registers
the shared type in its own composition root. The provider is a **published
artifact**, not a source checkout.

That distinction is already agreed in `devloom/.ai-rules/shared-ownership.md:5`:
*"exact checksum-pinned portable package artifacts with consumer conformance
and an invocation-supplied feed. Disable ambient platform source
auto-adoption; do not commit absolute sibling ProjectReferences. Missing
artifacts fail restore/preflight."*

A grant therefore permits an edit that consumes a **published artifact** and
refuses an edit that creates a **source-level coupling**. The old
"do-not-edit-siblings" rule could not tell those apart, which is precisely
why it blocked the safe case and the unsafe case alike.

## 4. Grant shape

Declared in the consumer's own `.project.json`, so the authorization is
reviewed in the repository whose files it authorizes:

```json
"adoption_grant": {
  "capability": "idempotency",
  "provider": "dotnet-platform-libs",
  "package": "Platform.Idempotency",
  "actor": "agent:<session-id>",
  "issued_at": "2026-10-02T00:00:00Z",
  "expires_at": "2026-10-09T00:00:00Z",
  "write_set": [
    ".project.json",
    "Directory.Packages.props",
    "src/**/CompositionRoot.cs"
  ],
  "rollback_ref": "openspec/changes/<change-id>"
}
```

`capability`, `provider`, `package`, `actor`, `issued_at`, `expires_at`, and
`write_set` are required. `expires_at` is required specifically so a grant
cannot silently become standing authorization. `write_set` is a list of
repository-relative glob patterns and MUST resolve inside the consumer root.

Lifecycle: `issued → active → consumed | expired | revoked`. `consumed` is
terminal and single-use: a second repoint under the same grant is
`ADOPTION_GRANT_SCOPE_VIOLATION`, because the permission was sized for one
change.

## 5. What the grant permits

Every permitted write resolves inside the **consumer** root:

| Permitted | Why it is inside the boundary |
|---|---|
| `capabilities[].consumes[]` in the consumer's `.project.json` | The consumer's own declaration of what it consumes. |
| The consumer's package manifest (`Directory.Packages.props`, `*.csproj`, `package.json`, `Cargo.toml`, `pyproject.toml`) | A pinned coordinate is a consumer-side statement, not a provider change. |
| The consumer's DI registration and call sites | Replacing a local helper with the shared one is the adoption itself. |
| The consumer's tests | Including the conformance check that proves no cross-repo reference exists. |
| The consumer's `AGENTS.md` | To record the adoption and the decline, if any. |

## 6. What the grant forbids

| Forbidden | Finding |
|---|---|
| Any write outside the consumer root, including a sibling `AGENTS.md` | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| Any write to the provider repository, at all | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| Bumping a provider version, or changing a provider's manifest | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| A `ProjectReference`, relative path, or `../` dependency on a sibling checkout | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| Promoting to `deployable: true` or writing `deployment/promotions.json` | `ADOPTION_GRANT_SCOPE_VIOLATION` |
| Using an expired, revoked, or already-consumed grant | `ADOPTION_GRANT_STALE` |
| Consuming a provider that is not `verified` with a resolving evidence ref | `ADOPTION_GRANT_PROVIDER_UNVERIFIED` |
| A repoint with no adoption record, or a hand-edited record | `ADOPTION_GRANT_UNSATISFIED` |

The version-bump prohibition is the sharpest edge and worth stating
plainly: adopting `Platform.Idempotency` **0.1.0** is permitted; asking for
0.2.0 because the consumer needs a method 0.1.0 lacks is not. That request is a
provider change, and it needs a provider change's authorization. A grant that
could force a version bump would be a grant to modify any provider in the
workspace, wearing a capability's name.

## 7. How the grant is checked

`validate_adoption_grant()` runs per project and is read-only, like every
other audit check:

1. **Parse** `adoption_grant`; a malformed block is a shape ERROR and no
   write is validated further.
2. **Resolve identity.** `provider` must be a registered project id.
   `capability` must already be bound to that provider in the capability
   registry. A grant that names an unbound capability is refused — the grant
   cannot be the thing that creates a binding.
3. **Resolve the window.** `expires_at` must be in the future relative to the
   audit. A past grant is `ADOPTION_GRANT_STALE`; the repoint it authorized
   is not retroactively legal.
4. **Resolve the provider.** The provider's declared capability must be
   `verified` with a resolving `evidence_ref`, else
   `ADOPTION_GRANT_PROVIDER_UNVERIFIED`. There is nothing lawful to adopt
   from an unverified provider.
5. **Check single use.** A consumed grant is `ADOPTION_GRANT_STALE` for any
   second repoint.
6. **Check the record.** If the consumer declares `consumes[]` for the
   granted capability, an adoption record must exist in
   `adoption/adoptions.json`; otherwise `ADOPTION_GRANT_UNSATISFIED`. A
   record whose content hash does not match the ledger's is refused, the same
   posture as a hand-edited `deployable: true` (`DEPLOYMENT_PROMOTION`).

The checker does **not** inspect build output or run a test. It proves an
authorization existed, was in scope, and produced a recorded, unmodified
outcome. Whether the repoint is *correct* is the consumer's own gate — which
is the same boundary the rest of this repo holds: governance records, product
gates verify.

## 8. The adoption record

`adoption/adoptions.json` is append-only, mirroring
`deployment/promotions.json`:

```json
{
  "schema_version": 1,
  "records": [
    {
      "consumer": "some-product",
      "capability": "idempotency",
      "provider": "dotnet-platform-libs",
      "package": "Platform.Idempotency",
      "version": "0.1.0",
      "replaced": ["src/Infrastructure/IdempotencyGuard.cs"],
      "actor": "agent:<session-id>",
      "grant_expires_at": "2026-10-09T00:00:00Z",
      "rollback_ref": "openspec/changes/<change-id>",
      "recorded_at": "2026-10-05T12:00:00Z",
      "content_hash": "<sha256 of the record body>"
    }
  ]
}
```

Six fields make adoption auditable and are not optional: **what** was
replaced (`replaced`, the local paths removed), **what** replaced it
(`provider`/`package`/`version`, pinned), **who** did it (`actor`), **under
what authority** (`grant_expires_at`), **how to undo it** (`rollback_ref`),
and **when** (`recorded_at`).

Rollback is a **second record** with `kind: "rollback"`, never an edit or a
deletion of the first. Tantalyn's pilot already committed to this discipline
— it documented "adoption and rollback boundaries" together — and an adoption
ledger that can be edited is an adoption ledger nobody can trust.

`content_hash` covers the record body. A hand-edited record fails the hash
and is refused; a hand-*added* record is a schema error against the ledger's
own append discipline, reported with the same code so the two cannot be
confused.

## 9. The `AGENTS.md` amendment

The clause to replace, verbatim from `devloom/AGENTS.md:17`:

> Assess shared infrastructure before duplication; see
> `.ai-rules/shared-ownership.md`. Do not edit siblings without scope
> authorization.

Two sentences, two contradictory instructions, one unreachable outcome.
The amendment keeps the first sentence's intent, makes the second precise,
and adds the single carve-out:

> **Assess shared infrastructure before duplication**; see
> `.ai-rules/shared-ownership.md`. Assess a named sibling or a published
> package for an existing capability before writing a local implementation,
> and name the specific sibling, project id, or package you assessed — an
> un-named assessment is not an assessment.
>
> **Do not edit a sibling repository without scope authorization.** This
> restriction governs *source* changes to another repository. It does not
> govern *consuming* a sibling's published artifact: a pinned, checksum-
> verified package coordinate, its registration in the consumer's own
> composition root, and the consumer's own tests are consumer-side changes.
> Under a valid `adoption_grant` in the consumer's `.project.json`, replacing
> a local implementation with a registered shared provider is authorized
> without separate cross-repo approval, and must be recorded in the
> consumer's `capabilities[].consumes[]` and in `adoption/adoptions.json`.
> A source-level coupling — a `ProjectReference`, a relative path, or a
> `../` dependency on a sibling checkout — remains forbidden regardless of
> any grant, as does bumping a provider's version.

What changed, precisely:

| | Before | After |
|---|---|---|
| "Assess shared infrastructure" | no named target, no completion criterion | must name the specific sibling, project id, or package assessed |
| "Do not edit siblings" | blocks source edits *and* published-artifact consumption | blocks source edits only |
| Carve-out | none | one: registered provider repoint under a grant, recorded in two places |
| Unchanged | — | source-level coupling, provider version bumps, promotion, ambient auto-adoption |

The same edit closes the second gap the 5 sibling-assessing repos have.
`therapist-commons/.ai-rules/workflow.md:22-25` currently reads *"A sibling
`common`/`manager` or platform project must be assessed for recurring
concerns; keep the capability local when ownership, privacy, lifecycle, or
dependency direction makes that the correct boundary."* That sentence is
**kept** — it is a correct boundary argument and this change must not
dismantle a legitimate reason to stay local. It gains only the requirement to
**name** the assessment, and the vocabulary it already uses (`ownership`,
`privacy`, `lifecycle`, `dependency direction`) becomes the closed decline
vocabulary rather than an ad-hoc justification.

## 10. Finding codes

New family, `ADOPTION_GRANT_*`, distinct from the gate change's
`CAPABILITY_ADOPTION_*` so two independent changes are not coupled by naming.

| Code | Level | Meaning |
|---|---|---|
| `ADOPTION_GRANT_SCOPE_VIOLATION` | ERROR | A write resolved outside the consumer root, or into the provider, or crossed a source-level coupling. The violation is the finding, independent of the resulting code state. |
| `ADOPTION_GRANT_UNSATISFIED` | ERROR | A declared `consumes[]` has no matching adoption record, or a record's `content_hash` does not match. |
| `ADOPTION_GRANT_STALE` | WARNING | The grant is expired, revoked, or already consumed. |
| `ADOPTION_GRANT_PROVIDER_UNVERIFIED` | WARNING | The provider capability is not `verified` or its `evidence_ref` does not resolve; the grant has nothing lawful to adopt from. |
| `ADOPTION_GRANT_SHAPE` | ERROR | The `adoption_grant` block is malformed, missing a required field, or carries a `write_set` pattern that escapes the consumer root. |

## 11. Compatibility and non-interference

- All 81 declarations lack an `adoption_grant` block and produce no
  `ADOPTION_GRANT_*` finding. Zero new ERRORs on a clean fleet.
- The `AGENTS.md` amendment is additive in effect: it grants one new
  permission, tightens the "name your assessment" requirement, and removes
  no existing restriction.
- `registry-integrity` is unaffected: no `id`, `path`, `adoption`, or
  `lifecycle` value changes.
- The promotion ledger is not extended, reused, or modified. Adoption is not
  promotion and is never a path to `deployable: true`.
- The Forge adapter is untouched; the new findings ride the existing
  `findings` list.

## 12. Verification oracle

Fixtures for every permitted write class (declaration, manifest, wiring,
test, `AGENTS.md`) asserting acceptance, and every forbidden write class
(provider source, provider version bump, cross-repo `ProjectReference`,
sibling `AGENTS.md`, promotion ledger) asserting
`ADOPTION_GRANT_SCOPE_VIOLATION`. Grant lifecycle fixtures: issued, consumed,
expired, revoked, and single-use enforcement. Ledger fixtures: a
hand-edited record refused on `content_hash`, a hand-added record refused on
append discipline, and a rollback recorded as a second row with the first
row intact. A fixture proving the audit itself spawns no child process and
writes nothing outside the governance root.

## 13. Decision ledger

Resolved: the grant is declared in the consumer's own `.project.json`; it is
single-use and must expire; the write set is consumer-root only; the
provider is read-only at all times; version bumps and source-level coupling
are forbidden outright; the record is append-only with a `content_hash` and
a rollback row; the `AGENTS.md` clause splits source edits from published-
artifact consumption; the decline vocabulary in the sibling-assessing rule is
preserved, not replaced.

Deferred: how a grant is issued (a reviewing declaration, a CLI
(`scripts/grant_adoption.py`), or a signed token); the grant duration; any
first adoption executed under the grant; consolidating the 58
`WebApplicationFactory` subclasses or the 12 hand-rolled Postgres fixtures,
each of which needs its own grant.

Blockers: none. The grant is a permission model and is valid on its own. It
does not require `capability-adoption-gate`, and that gate does not require
this change — the two compose, and this change reads the
`capabilities[].consumes[]` declaration as an ordinary optional field when
the gate is absent.
