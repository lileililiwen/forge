# Toolchain Decision Register

This document is the human view of the toolchain decisions resolved by the
workspace. The machine-readable source of truth is `decisions.json` at
the governance root; this file restates each entry in prose so reviewers
do not have to read JSON. Both stay in sync because the loader enforces
the shape and `tests/test_decisions.py` asserts the D2 `policy_boundary`
text, the D3 ownership split and the D9 provider-neutral policy.

| ID | Title                                       | Status     | Date       | Owner                  |
|----|---------------------------------------------|------------|------------|------------------------|
| D1 | Agent runtime of record                     | accepted   | 2026-09-30 | workspace-governance   |
| D2 | Self-hosted identity provider               | superseded | 2026-09-30 | workspace-governance   |
| D3 | Capability registry home                    | accepted   | 2026-09-30 | workspace-governance   |
| D4 | Crate publication mode                      | accepted   | 2026-09-30 | workspace-governance   |
| D5 | The rules drift report stays unwired        | accepted   | 2026-10-03 | workspace-governance   |
| D6 | Project identity is immutable; two axes     | accepted   | 2026-10-03 | workspace-governance   |
| D7 | Unpinned interface edges ramp to FAIL       | accepted   | 2026-10-17 | workspace-governance   |
| D8 | Python security audit audits the project    | accepted   | 2026-10-17 | workspace-governance   |
| D9 | Provider-neutral workspace identity policy  | accepted   | 2026-10-19 | workspace-governance   |

Inspect or script the register with:

```bash
python3 scripts/workspace_check.py --decisions
python3 scripts/workspace_check.py --decisions --json
```

## D1 — Agent runtime of record

Adopt a declared split between the two agent runtimes: **ariadex** owns
interactive long-running developer sessions, while **sisyphusfy** owns
headless per-task loops with independent verification. Each runtime is
registered in `projects.json` and ships its own profile; the split is a
workspace-level policy so the two runtimes are not interchangeable and
shared code is forbidden across them without an explicit supersede.

## D2 — Self-hosted identity provider (superseded by D9)

Adopt **Zitadel** as the self-hosted identity provider, run as a shared
workspace runtime on `jenkins-local`, and exposed to product repositories
as the single workspace SSO seam.

**Status.** `superseded` on 2026-10-19 by **D9**. The text below is the
historical record and is not rewritten. D9 keeps the internal-workspace
boundary and the public-site exclusion and replaces the mandatory vendor
with provider-neutral standard OIDC policy.

**Policy boundary.** The shared identity provider covers internal
workspace products only. The public GitHub Pages catalog
(`lileililiwen-site` and any sibling published as a static site) remains
**excluded** from shared login and must not consume workspace SSO, shared
analytics, or any cross-product identity data. This exclusion restates the
root `HANDOFF.md` ownership table and is the binding interpretation of
this decision.

**Downstream references.** One registered OpenSpec change is named so far:
`dotnet-platform-libs` is the home of the `identity-oidc-client` change
that proves the OIDC client seam in a C# product. The runtime profile and
per-product OIDC client changes are still planned in B3 of the
`TOOLCHAIN_ROADMAP.md` and will be added to the register when each is
authored.

## D3 — Capability registry home

Split the capability registry between two owners. **`platform-contracts`**
owns the entry schema (the shape of a capability declaration).
**`workspace-governance`** owns the registered instances (the live records
of which projects declare which capabilities, kept next to `projects.json`).

**Policy boundary.** Capability entry schemas are owned by
`platform-contracts`; registered capability instances are owned by
`workspace-governance`. Consumers must read the schema from
`platform-contracts` and the instances from this repository; the two are
never merged into a single file.

**Downstream references.** `platform-contracts: capability-registry-v1`
is the schema change that backs this decision. It is still planned in B4
of `TOOLCHAIN_ROADMAP.md` and is recorded here so the audit can flag a
silently rewritten ownership split.

## D4 — Crate publication mode

Publish the `rust-platform-libs` crates to **crates.io** instead of
vendoring them as path dependencies. The repository is Apache-2.0, so the
legal preconditions for public publication are satisfied; consumer
adoption migrates off path dependencies only after the new
`registry-publication` change lands.

**Downstream references.** `rust-platform-libs: registry-publication` is
the prerequisite change that records the version policy, CI publishing
secret, `publish = false` flip, and name-squat check.

## D6 — Project identity is immutable; classification is two axes

A project is identified by an immutable registry `id` plus two independent
classification axes: `role` (what the repository is for) and `stack` (what it
is written in). Both are carried by the registry entry and by the declaration,
and a disagreement between the two surfaces is `TAXONOMY_MISMATCH`.

`kind` is retired as an authored value and retained permanently as a derived,
deprecated composite — `<stack>-<role>`, or the bare `<role>` when the stack is
`none` — so every consumer of the old string keeps parsing. `shared-library` is
not a seventh role: `platform-contracts`, `rust-platform-libs` and
`dotnet-platform-libs` take `role: platform`. `stack` stays single-valued.
`lileililiwen-site` takes `role: deployment` and keeps its id, because it
resolves through the registry `path` `lileililiwen.github.io`.

`TAXONOMY_MISMATCH` is ERROR and was not enabled until the migration was
applied; the migration has now landed and the committed constant
`TAXONOMY_AUDIT_ENABLED` is `True`. The severity was never softened to
WARNING, and enabling it early would have turned the gate red on every
unresolved row — which is what the checker itself proved, in both directions of
the flag, before the flip. The migration itself is a fleet-wide cross-repo
source edit, applied by `scripts/axis_migration.py`. The full text, the
measured 30-of-77 disagreement it resolves, and the immutability rule are in
`decisions.json`; the mechanism is in `docs/vocabulary.md`.

## D7 — Unpinned interface edges escalate to FAIL on 2027-01-15

`INTERFACE_SIBLING_PATH_UNPINNED` and `INTERFACE_VENDORED_WITHOUT_DIGEST` are
reported at WARNING, and become blocking from **2027-01-15**. The date is
recorded in the entry's `escalates` field, which is where the audit reads it,
so the ramp is an owner decision that stays visible and revocable rather than a
silent tightening — nothing becomes blocking before the fleet has had a release
cycle in which to declare its interfaces, and no environment variable or
project-controlled flag can start it early or hold it back.

`mechanism: sibling` — the relative-path invocation a repository's gate uses to
reach another checkout — is lawful only when the consuming project also
declares that provider `required: true` and pins the edge with a digest of the
invocation it wrote. The two lawful alternatives to an unpinned sibling edge
are a digest-pinned copy declared as `mechanism: consume`, or a published
artifact consumed by version. The declaration, the mechanism set and the
finding vocabulary are in `docs/capabilities.md`.

## D8 — A Python security audit audits the project, not the host interpreter

`pip-audit` on a bare environment reports advisories for packages the
repository never declared. The derived security concern must audit the
project's own declared dependency set and must be left **UNBOUND with a
reason** — never a passing or host-scoped substitute — when the repository
declares no Python dependency set. The scoping change is recorded as
`workspace-governance: python-security-audit-scoping` and the rebinding of
the affected repositories as `rebind-security-concern`.

## D9 — Provider-neutral workspace identity policy

Standard **OIDC** is the interoperability protocol for workspace identity.
Compatible identity providers are **replaceable**, and no vendor is required
by policy. This decision supersedes D2 without rewriting it.

**Identity populations.** Workspace administrators may authenticate through
one operator-selected workspace OIDC provider, but that authentication grants
no application permission by itself: each application grants access
explicitly. Product end users are a **distinct population per product** and
are project-scoped by default, with no account, session, role or permission
shared implicitly across projects. Configuring one product end-user
population for more than one product is refused by default.

**Policy boundary.** Shared workspace SSO is internal-only and stays limited
to internal workspace products. The public GitHub Pages catalog
(`lileililiwen-site` and any sibling published as a static site) remains
**excluded** from shared login and must not consume workspace SSO, shared
analytics, or any cross-product identity data.

**No runtime migration.** The operator's present provider deployment remains
the current configuration until a separately authorized runtime change.
This decision declares no migration and makes no claim about deployment
health.

**Downstream references.** `dotnet-platform-libs: project-oidc-auth-adoption`
provides the reusable ASP.NET Core OIDC adapter, and
`forge: forge-auth-provider-extensions` provides the opt-in project setup
journey with replaceable provider packs. Both consume this policy; neither
redefines it.

## Register rules

1. The register is owned by `workspace-governance` and lives at
   `decisions.json` at the governance root.
2. `workspace_check.py --self-check` fails closed if the file is missing
   (`DECISIONS_REGISTER_MISSING`).
3. Ids are permanent. Changing an accepted decision requires a NEW entry
   whose predecessor is marked `superseded`; the old text stays in place
   so the historical record is intact.
4. `status` is one of `proposed`, `accepted`, `rejected`, `superseded`.
5. `owner_project` and every `downstream[].repo` MUST be a registered
   `projects.json` id; `downstream[].change` MUST be kebab-case.
6. The D2 `policy_boundary` field is machine-checkable: a test asserts
   that the public-catalog exclusion is present. Editing the decision
   without keeping the exclusion will fail the test.
7. The D9 `policy_boundary` is machine-checkable: tests assert that D9
   keeps the public-catalog exclusion, stays internal-only, separates
   workspace administrators from product end users, mandates no vendor,
   and declares no runtime migration.
8. No code in this repository edits the register outside a code review;
   history integrity is git review plus the immutability rule, not
   runtime enforcement.
