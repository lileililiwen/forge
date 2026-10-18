# Proposal: Grant bounded cross-repo capability adoption

## Why

`capability-adoption-gate` makes non-consumption of a registered shared
capability a measured finding. Measuring duplication is half the problem. The
other half is that **no agent in this workspace is currently allowed to fix
it**, and the rule that forbids the fix is the same rule that invites it.

The contradiction is quoted in `devloom/AGENTS.md:17` and carried by every
sibling clone of the shared rules clause:

> Assess shared infrastructure before duplication; see
> `.ai-rules/shared-ownership.md`. **Do not edit siblings without scope
> authorization.**

Read the two sentences together. "Assess shared infrastructure before
duplication" instructs the agent to prefer the shared implementation.
"Do not edit siblings without scope authorization" makes adopting one
impossible, because adoption of `Platform.Idempotency` into a product
requires touching the product's package manifest *and* deciding how the
provider behaves — and the provider is a sibling. An agent that reads both
sentences and obeys both concludes that writing a local idempotency helper is
the **only compliant outcome**. It is not a knowledge failure. It is the
rational response to an unsatisfiable rule.

That is why 34 repos touch idempotency while 2 consume it, 28 touch rate
limiting while 2 consume it, and 28 touch logging/redaction while 0 consume
it, even though `Platform.Web.Telemetry` exists. Six further repos
(`therapist-commons`, `careBridge`, `kairovia`, `citylens`, `retakeKit`,
`trailCrew`) instruct agents to *"assess sibling `common`/`manager`
projects"* without naming which ones, so the assessment has no defined
target and no completion criterion.

Three further facts make the permission, not the measurement, the binding
constraint:

1. **The providers are published and pinned.** Five repos already restore
   `Platform.*` 0.1.0 coordinates from a local feed recorded in
   `Directory.Packages.props` — `therapist-commons` pins eight packages
   including `Platform.Idempotency`, `careBridge` pins six including
   `Platform.Identity.AspNetCore`. The consumption path is already proven
   to work; only the authorization is missing.
2. **The safe boundary is already known.** `devloom`'s own
   `.ai-rules/shared-ownership.md:5` records the constraint that any grant
   must honour: *"exact checksum-pinned portable package artifacts with
   consumer conformance and an invocation-supplied feed. Disable ambient
   platform source auto-adoption; do not commit absolute sibling
   ProjectReferences. Missing artifacts fail restore/preflight. Existing
   shared code is not reimplemented locally."* That is a precise,
   already-agreed description of what a lawful adoption looks like.
3. **One pilot has proven the model end to end.** Tantalyn's archived
   change `2026-09-09-platform-consumer-pilot` documents consuming selected
   published artifacts, proving no cross-repository `ProjectReference`
   exists, and documenting adoption *and rollback* — the exact shape this
   grant formalises for the general case.

## What Changes

- Add a **capability-scoped adoption grant**: a declared, expiring
  permission for one named agent session to repoint one consumer onto one
  registered provider for one capability, without cross-repo authorization.
- Amend the `AGENTS.md` shared-rules clause. The "assess before duplicating"
  sentence and the blanket "do not edit siblings" sentence are replaced by a
  single rule that distinguishes a **consumer-side repoint** (permitted
  under a grant) from a **sibling mutation** (still forbidden). This is the
  only change that alters agent behaviour; everything else records and
  checks it.
- Define the grant's **exact write set**: the consumer's own `.project.json`,
  its own package manifests, its own DI registration and call sites, and its
  own tests. Define the **exact forbidden set**: the provider repository,
  provider version bumps, cross-repo relative or `ProjectReference`
  dependencies, promotion to `deployable`, the promotion ledger, and every
  other project root.
- Produce an **adoption record** for every granted change: what was
  replaced, the pinned coordinates before and after, the evidence, the
  actor, and the rollback. Appended to a tamper-evident ledger
  `adoption/adoptions.json`, mirroring the promotion-ledger pattern, so
  adoption is auditable and reversible on paper.
- Add the grant checks to `workspace_check.py` under four new stable
  identifiers: `ADOPTION_GRANT_SCOPE_VIOLATION`, `ADOPTION_GRANT_UNSATISFIED`,
  `ADOPTION_GRANT_STALE`, `ADOPTION_GRANT_PROVIDER_UNVERIFIED`. A grant
  that was exercised outside its scope is an ERROR regardless of the
  resulting code state, because the violation is the finding.
- Keep the `capabilities[].consumes[]` declaration the single source of what
  a repo claims to consume, so this change and the adoption gate read one
  record rather than two.

## Package Boundary and Split Assessment

This package ends at a **bounded permission with an auditable record**. It
does not itself migrate any repo, does not change any provider, and does not
enforce that adoption happened. Enforcing adoption is the separate
`capability-adoption-gate` change, which measures and escalates; this change
only makes the lawful action available and recorded. The split is the point:
a gate that measures duplication without a lawful path to remove it converts
a documentation problem into a blocked fleet, and a grant without a gate
converts an incentive problem into untracked churn.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `cross-repo-adoption-grant` | An agent may repoint a consumer onto a registered provider under a bounded, recorded grant | Workspace Governance, Python 3 stdlib | `adoption_grant` in the consumer's `.project.json`; `adoption/adoptions.json` ledger; `ADOPTION_GRANT_*` findings | registered provider id; existing promotion-ledger pattern | A grant permits exactly the consumer-root write set and refuses every provider-root and version-bump write |
| `capability-adoption-gate` | Non-consumption of a registered capability is a measured finding | Workspace Governance, Python 3 stdlib | `capabilities.json`, `capabilities[].consumes[]`, `CAPABILITY_ADOPTION_*` | existing `validate_capabilities` | 34/28/21/15-repo duplication resolves to a finding or a decline |

**These two changes are independent.** Neither requires the other to be
valid, complete, or even present. This change is a permission model: it
defines who may write what, under which conditions, with which record — all
of which is meaningful whether or not a gate measures the result. The gate
change is a measurement: it defines what counts as non-adoption and how it
escalates — all of which is meaningful whether or not any agent is permitted
to act on it. They are two ends of one loop, and the loop is closed by
composition rather than by a dependency. Neither proposal's acceptance
criteria reference the other. The shared surface is the
`capabilities[].consumes[]` declaration, which the gate change introduces and
this change reads; where the gate change is absent, this change treats
`consumes[]` as an ordinary optional additive field.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `promote_project.py` + `deployment/promotions.json` | `scripts/promote_project.py`; `README.md` "Deployment promotion" | The repo's only precedent for a recorded, tamper-evident, append-only transaction ledger with a script that validates and refuses hand-edits | Ledger records a promotion, not a cross-repo repoint | Workspace Governance release | **adopt the pattern** verbatim for `adoption/adoptions.json`; do not extend the promotion ledger itself |
| `standalone-project-boundary` | `openspec/specs/standalone-project-boundary/spec.md` | Existing precedent for classifying each project and reporting `workspace-dependent` when a sibling checkout is required or directly imported | Says a sibling dependency is a fact to report; says nothing about who may create one | Workspace Governance release | **extend shared owner** — a granted repoint is a `workspace-dependent` fact with an authorization attached |
| Canonical shared rules clause | `devloom/AGENTS.md:17`; `therapist-commons/.ai-rules/workflow.md:22-25` | The single clause that invites reuse and forbids adoption; 9+ repos carry it | Self-contradictory as written; no named target, no completion criterion | Each repo, via the shared rules baseline | **amend the rule text** — the only behavioural change in this package |
| `shared-ownership.md` constraints | `devloom/.ai-rules/shared-ownership.md:5` | Already-agreed adoption boundary: checksum-pinned portable artifacts, invocation-supplied feed, no ambient source auto-adoption, no absolute sibling `ProjectReference`, missing artifacts fail restore/preflight | Describes the constraint; never grants permission to a specific session | Each repo | **adopt verbatim as grant preconditions** — do not invent a second boundary |
| Tantalyn platform consumer pilot | `tantalyn/openspec/changes/archive/2026-09-09-platform-consumer-pilot/` | A worked example: scoped consumption, a consumer-conformance check proving no cross-repo `ProjectReference`, and documented adoption *and* rollback | One repo, one pilot, no reusable permission model | Tantalyn | **adopt the pilot's shape** as the record template; do not re-run the pilot |
| `dotnet-platform-libs` | `Directory.Packages.props` across `therapist-commons`, `careBridge`, `tantalyn`, `citylens`, `devloom` | Published 0.1.0 coordinates already pinned by five consumers; the local feed is already the delivery mechanism | No governance-side record of who consumed what, when, under what authority | dotnet-platform-libs release | **reference as the provider side; never modify it** — the grant may not bump or edit a provider |
| `capability.schema.json` (platform-contracts) | `platform-contracts/schemas/capability.schema.json` | Versioned contract family for capability declarations | No consumer-side adoption or authorization record | platform-contracts release | **reference, do not fork** — the grant field set is additive and governance-owned |
| `driftwatchdog` gate runtime | `driftwatchdog` repo, `.ai-gate/gate.yaml` | Shared quality/evidence gate runtime; could execute an adoption check | A grant is a governance record, not a gate check; 76 of 76 repos declare `tests: true` but only 6 bind a command | driftwatchdog release | **defer** — command binding is a separate change owned by the `driftwatchdog` repository; referenced, not duplicated |
| `.ai-gate/gate.yaml` note field | `workspace-governance/.ai-gate/gate.yaml:16` | Existing precedent for a note stating a declaration does not constitute a run | — | Workspace Governance release | **adopt the posture** — a grant authorizes, it never certifies |

## BFS Impact Map

- **Actors:** a granted agent session (the only principal the grant binds),
  the consumer repository (the only root it may write), the provider
  repository (never written), the registry owner (issues and revokes
  grants), and the auditor (validates scope and records).
- **State:** a grant moves `issued → active → consumed | expired | revoked`.
  An adoption record is `recorded` once and immutable; a rollback is a second
  record, never an edit of the first. A grant is single-use: consuming it
  moves it to `consumed` and it cannot back a second repoint.
- **Persistence:** the grant is declared in the **consumer's own**
  `.project.json` under an additive `adoption_grant` block, so the
  authorization lives next to the change it authorizes and is reviewed in
  the consumer's own repository. The executed outcome is appended to
  `adoption/adoptions.json` at the governance root. **No change to
  `projects.json`, no new migration, no new SQLite state, and the promotion
  ledger is not touched.**
- **Security:** the grant names exactly one consumer id, one capability id,
  one provider id, and a bounded file set. Every write must resolve inside
  the consumer root; a path escaping that root is `ADOPTION_GRANT_SCOPE_VIOLATION`
  and the file is not read or written. Provider roots are checked by identity
  before any write. The existing `vocabulary.json` secret-substring
  redaction applies to every recorded field. The grant authorizes a
  repository mutation; it does not authorize execution, deployment,
  promotion, or publication, and it never carries a credential.
- **Failure:** an expired, revoked, already-consumed, provider-unverified,
  or out-of-scope grant is refused with a typed finding and writes nothing.
  A hand-edited adoption record is refused, exactly as a hand-edited
  `deployable: true` is refused today with `DEPLOYMENT_PROMOTION`. A
  missing record for a completed repoint is `ADOPTION_GRANT_UNSATISFIED`.
- **Compatibility:** `.project.json` without an `adoption_grant` block is
  valid and produces no finding, so all 81 declarations are unaffected. The
  `AGENTS.md` amendment is additive in effect: it grants one new permission
  and removes no existing restriction. No exit code, report key, finding
  code, or adapter observation key changes.
- **Verification:** fixtures for each permitted write (manifest, wiring,
  test, declaration), each forbidden write (provider source, provider version
  bump, cross-repo reference, sibling `AGENTS.md`, promotion ledger), grant
  lifecycle fixtures (issued, consumed, expired, revoked, single-use
  enforcement), and a tamper fixture proving a hand-edited record is refused.

## Capabilities

- **New:** `cross-repo-adoption-grant`.
- **Modified:** `platform-capability-manifest` only through the additive
  `adoption_grant` declaration and the `consumes[]` record it produces.
  `registry-integrity` is unaffected: a grant changes no `id`, `path`,
  `adoption`, or `lifecycle` value.

## Non-goals

- No edit, publish, version bump, or retarget of any provider repository.
- No creation of a new capability or provider, and no change to the
  capability vocabularies owned by `platform-contracts` or hermora.
- No promotion to `deployable: true` and no entry in the Jenkins manifest.
- No ambient source auto-adoption, no cross-repository `ProjectReference`,
  and no absolute sibling paths — those remain forbidden by
  `shared-ownership.md` and this grant does not relax them.
- No migration of any repo in this change. The grant is the permission; the
  first adoption executed under it is a separate, individually reviewed
  change.
- No consolidation of the 58 `WebApplicationFactory` subclasses or the 12
  hand-rolled Postgres fixtures. Those are follow-on work, each requiring
  its own grant.

## Open decisions deliberately left to the owner

1. **How a grant is issued.** Three candidates: (a) a human-written
   `adoption_grant` block in the consumer's `.project.json` reviewed in that
   repo; (b) a CLI
   (`scripts/grant_adoption.py`) that writes the block after a confirmation;
   (c) a signed token. This proposal specifies (a) because it needs no new
   trust anchor and is reviewable where the change lands, and names (b) as
   the likely follow-up. The choice is the owner's.
2. **Grant lifetime.** `expires_at` is required by this proposal so a grant
   cannot become a permanent standing authorization. The duration itself is
   an owner decision; the requirement that one exists is not.
3. **The finding-code namespace.** This proposal uses `ADOPTION_GRANT_*` as a
   family distinct from the gate change's `CAPABILITY_ADOPTION_*`, so the
   two independent changes are not coupled by naming. If the owner prefers a
   single shared `ADOPTION_*` namespace, both change names must be edited
   together.
