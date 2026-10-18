# Proposal: Measure shared-capability adoption instead of assuming it

## Why

The workspace owns a `capabilities` block. Nobody uses it. It is populated in
**2 of 81** declarations — `forge` (7 entries) and `workspace-governance`
(4) — while `scripts/workspace_check.py` validates only its *shape*. The
checker's own docstring says it "does not promote a capability to verified on
its own", which is correct, but it means the block can never detect the
failure it was written to prevent.

The duplication that block was meant to prevent is already present and
measured:

| Capability | Repos that need it | Repos that consume the provider |
|---|---|---|
| idempotency (`Platform.Idempotency`) | 34 | 2 |
| rate limiting | 28 | 2 |
| logging / redaction (`Platform.Web.Telemetry`) | 28 | 0 |
| health checks | 21 | 0 |
| CSRF / antiforgery | 15 | 0 |
| feature flags (`Platform.FeatureManagement`) | 0 | 0 |
| Postgres test fixture (`Platform.Testing` has no DB fixtures) | 12 | 0 |
| `WebApplicationFactory` harness (one shared 98-line factory exists) | 58 subclasses across 25 repos | — |

The incentive behind the numbers is a rule, not a knowledge gap. Workspace
`AGENTS.md` files instruct agents to *"Assess shared infrastructure before
duplication; see `.ai-rules/shared-ownership.md`. Do not edit siblings without
scope authorization."* Assess invites reuse; do-not-edit forbids adoption.
Under that rule, hand-rolling a local idempotency helper is the **only
compliant outcome** — the agent cannot change the provider, and adopting one
would require authorization it does not have. Five further repos
(`therapist-commons`, `careBridge`, `kairovia`, `citylens`, `retakeKit`,
`trailCrew`) instruct agents to *"assess sibling `common`/`manager` projects"*
without naming which ones, so the assessment has no defined target.

Two structural defects make the invisibility worse:

1. **The `capabilities` block is provider-shaped.** Its required fields are
   `owner` and `evidence_state` — *who provides this*, not *what I consume*.
   `docs/capabilities.md` says the block covers "consume **or** provide", but
   the schema has no field for a consumption edge. `forge` and
   `workspace-governance` use it to declare what they *give*. A consumer that
   wants to record an adoption has nowhere to write.
2. **"Capability" has three incompatible definitions**: the declarative block
   (`schemas/project.schema.json`), the contract family
   (`platform-contracts/schemas/capability.schema.json`), and hermora's
   endpoint list. The audit cannot compare a repo against a provider while
   three vocabularies disagree on what a capability *is*.

A prior review claimed "66/72 repos evidence_status: planned". That is false:
`projects.json` has no `evidence_status` field, and across `.project.json`
the real distribution is `implemented` 38, `verified` 33, `planned` 6. Any
ramp calibrated on that claim would have started the whole fleet at FAIL.
This change uses the measured distribution and re-derives nothing from the
claim.

## What Changes

- Add a **read-only, bounded adoption measurement** to
  `scripts/workspace_check.py`: for every capability a repo is *expected* to
  consume, resolve the consumption edge from the consumer's own manifests
  (`Directory.Packages.props`, `*.csproj`, `package.json`, `Cargo.toml`,
  `pyproject.toml`, `requirements*.txt`) rather than from a hand-declared
  boolean. No project command is ever executed.
- Extend the `capabilities` block from **declaration** to a
  **declaration + evidence pair**: an additive `consumes` array per
  capability carrying the provider id, the pinned version, and a
  repository-relative `evidence_ref`. Additive — the 79 declarations without
  it stay valid.
- Add a **capability adoption registry** (`capabilities.json` at the
  governance root, parallel to `decisions.json` and `projects.json`) that
  binds each capability id to exactly one provider and assigns a posture:
  `expected` (WARN), `required` (FAIL), or `exempt`. `projects.json` is not
  modified.
- Add the **WARN→FAIL ramp** as a deterministic, recorded transition. A
  capability is `expected` from the day the registry admits it, and becomes
  `required` only when (a) the provider is published and `verified`, and
  (b) the capability has been `expected` for a declared number of consecutive
  audits. Graduations are recorded entries, not wall-clock heuristics.
- Add the **duplicate-implementation rule**: a repo that ships a local
  implementation of a capability it is expected to consume is a finding
  (`CAPABILITY_ADOPTION_MISSING`), because that is the exact shape of the
  34-repo idempotency split.
- Add a **decline record** so legitimate non-adoption is a first-class,
  reviewable answer rather than silence. A decline requires a rationale from a
  closed vocabulary and is exempt from the ramp until it is withdrawn.
- Declare four new finding codes in the existing `CAPABILITY_*` namespace:
  `CAPABILITY_ADOPTION_MISSING`, `CAPABILITY_ADOPTION_UNEVIDENCED`,
  `CAPABILITY_ADOPTION_EVIDENCE_STALE`, `CAPABILITY_ADOPTION_DECLINE_UNRECORDED`.
  An unresolved capability id is reported separately as
  `CAPABILITY_ADOPTION_UNRESOLVED` at REVIEW_REQUIRED level and never
  graduates to FAIL.

## Package Boundary and Split Assessment

This package ends at a **measured adoption finding with a recorded
disposition**. It does not authorize anyone to fix the finding. Repairing a
non-adoption requires editing a consumer's wiring — that permission problem
is the incentive contradiction, and it is owned by the separate
`cross-repo-adoption-grant` change. The split matters: a gate that measures
duplication without a lawful path to remove it converts a documentation
problem into a blocked fleet.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `capability-adoption-gate` | Non-consumption of a registered capability is a measured finding | Workspace Governance, Python 3 stdlib | `capabilities.json`, `capabilities[].consumes[]`, `CAPABILITY_ADOPTION_*` findings in `workspace_check.py` | existing `validate_capabilities`; provider publication evidence | Portfolios of 34/28/21/15/12-repo duplication resolve to a finding with a pinned version or an explicit decline |
| `cross-repo-adoption-grant` | An agent may repoint a consumer onto a provider under a bounded grant | Workspace Governance, Python 3 stdlib | `adoption_grant` declaration, `adoption/adoptions.json` ledger | registry + provider identity | A grant permits exactly the consumer-root writes and refuses every provider-root write |

Dependency order: this change is independently adoptable. It needs a
capability id to bind to a provider, and it needs a provider to be
`verified` before FAIL is reachable — **not** the grant. With the grant
absent, the gate still measures and still records; the remediation path is
simply not yet granted.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `validate_capabilities` | `scripts/workspace_check.py:332` | Shape validation, `CAPABILITY_OWNERS`, `CAPABILITY_EVIDENCE_STATES` | Validates shape only; never compares a consumer to a provider | Workspace Governance release | **extend shared owner** (additive `consumes`, no signature change) |
| `capability.schema.json` (platform-contracts) | `platform-contracts/schemas/capability.schema.json` | Versioned contract family for capability declarations | No consumer-side edge and no provider binding | platform-contracts release | **reference, do not fork** — read ids, never re-declare the shape |
| `projects.json` | governance root | 78 entries with `id`/`path`/`profile`/`adoption`/`lifecycle` | Carries identity, not capability identity | Workspace Governance release | **extend with a sibling file**, not in place |
| `decisions.json` | governance root; `docs/toolchain-decisions.md` | Precedent for a machine-readable decision record with tests asserting its presence | Decision register only | Workspace Governance release | **adopt the pattern** for `capabilities.json` + graduation entries |
| `deployment/promotions.json` | `scripts/promote_project.py` | Tamper-evident append-only ledger for a promotion transaction | Promotion is the only path to `deployable: true` | Workspace Governance release | **adopt the pattern** for adoption graduations (not for grants) |
| `driftwatchdog` gate runtime | `driftwatchdog` repo, `.ai-gate/gate.yaml` `checks:` block | Shared quality/evidence gate runtime | 76 of 76 repos declare `tests: true`, only 6 bind a command; `driftwatchdog` returns `NotApplicable`/Info | driftwatchdog release | **defer** — command binding is a separate change owned by the `driftwatchdog` repository; this change only emits findings to the existing report channel |
| hermora endpoint list | hermora repo | A third, endpoint-shaped notion of capability | Incompatible id space with both schemas | hermora release | **defer** — an endpoint list is not a capability registry |
| `product_code_quality.py` | `scripts/product_code_quality.py` | Additive opt-in gate with REVIEW_REQUIRED as a first-class outcome | Single-project; no cross-repo comparison | Workspace Governance release | **adopt the posture** (fail-closed to REVIEW_REQUIRED, never a guessed pass) |

## BFS Impact Map

- **Actors:** the governance auditor (read-only), the registry owner
  (records a graduation or a decline), a product repository (declares
  `consumes[]`), a platform repository (publishes and verifies a provider).
- **State:** a capability id moves `absent → expected (WARN) → required
  (FAIL)`, or `expected → exempt` through a recorded decline. Consumer-side,
  a `consumes[]` entry moves `declared → evidenced` when its
  `evidence_ref` resolves, and back to `declared` when it stops resolving
  (`CAPABILITY_ADOPTION_EVIDENCE_STALE`).
- **Persistence:** the new `capabilities.json` at the governance root holds
  provider bindings, postures, and graduation entries; `.project.json` gains
  one additive key. **No change to `projects.json`, no new migration, no new
  SQLite state.** Consumption edges are read from the consumer's own
  manifests, which already exist — for example five repos already pin
  `Platform.*` versions in `Directory.Packages.props`
  (`therapist-commons`, `tantalyn`, `careBridge`, `citylens`, `devloom`).
- **Security:** the measurement reads only the consumer's own files, bounded
  to the manifest list above, with the existing path-escape and
  secret-substring rules from `vocabulary.json` applied. No command
  execution, no network, no package-manager invocation, no write to any
  project. An absolute or `..`-bearing `evidence_ref` is refused, never
  normalized.
- **Failure:** a missing or unparseable `capabilities.json` is a governance
  ERROR, not an empty-registry fallback. An unresolved capability id, a
  provider that is not `verified`, an unreadable manifest, and a decline
  with a rationale outside the closed vocabulary are REVIEW_REQUIRED or
  WARNING — they never produce a FAIL and never claim adoption.
- **Compatibility:** declarations without `consumes[]` remain valid and
  produce no adoption finding. `projects.json`, existing `CAPABILITY_*`
  codes, exit codes, the Forge adapter's observation keys, and the
  `maturity_audit.py` single-result invariant are unchanged.
- **Verification:** schema fixtures for a valid and an invalid `consumes`
  entry; a portfolio fixture reproducing the 34/28/21/15 duplication split;
  a decline fixture for each rationale class; a read-only proof (recursive
  tree hash identical before and after); and a unit test asserting the ramp
  cannot jump straight to `required`.

## Capabilities

- **New:** `capability-adoption-gate`.
- **Modified:** `platform-capability-manifest` only through the additive
  `capabilities[].consumes[]` field. Its existing requirement — *declarations
  are intent, not implementation evidence* — is preserved and extended, not
  weakened: a consumption edge is still not a verification of behaviour.

## Non-goals

- No automatic dependency injection, package installation, or edit to any
  product or platform repository.
- No promotion to `deployable`, and no effect on the Jenkins manifest.
- No rewrite of `kind`. The `kind` conflation (31 `product`, 19
  `dotnet-product`, 7 `typescript-product`, 6 `rust-product`, alongside 6
  `tooling` and 3 `platform`) is a real vocabulary defect, but the registry
  misclassifying `forge` and `driftwatchdog` as rust-products while their
  own declarations say `platform`/`tooling` is the *taxonomy* problem, and
  this change must not inherit it by keying findings on `kind`.
- No enforcement of the `tests: true` gate check; that binding is owned by
  the `driftwatchdog` repository.
- No replacement for hermora's endpoint list, and no merge of the three
  capability vocabularies into one file.

## Open decisions deliberately left to the owner

1. **What evidence qualifies as proof of adoption.** This proposal requires
   a *machine-resolvable consumption edge* (a pinned `PackageVersion`,
   a `PackageReference`, an npm/Cargo/Python dependency) plus a declared
   `consumes[]` entry. It deliberately does **not** require behavioural
   evidence — no "the shared telemetry wrapper is actually registered" test
   signal. That stricter bar is available and is the owner's call, because it
   moves the gate from "is wired" to "is used correctly", and the second is
   not decidable from manifests.
2. **The finding-code namespace.** This proposal uses
   `CAPABILITY_ADOPTION_*`, extending the existing `CAPABILITY_*` family. The
   alternative is a fresh `ADOPTION_*` namespace shared with the grant change
   (which uses `ADOPTION_GRANT_*`). A single namespace was rejected because
   the two changes are independent and must not couple by naming.
3. **Who owns `capabilities.json`, and whether it merges into
   `projects.json`.** Decision `D3` splits capability *schema* (owned by
   `platform-contracts`) from *registered instances* (owned here). This
   proposal honours that split and adds a third artifact rather than
   renegotiating `D3`.
