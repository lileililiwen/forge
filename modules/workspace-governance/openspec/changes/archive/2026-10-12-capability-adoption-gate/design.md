# Design: Measure shared-capability adoption instead of assuming it

## 1. Implementation boundary

Repository `/home/paul/code/workspace-governance`, Python 3 standard library
only. No new dependency, no new runtime.

- **Extension points are exactly:** `scripts/workspace_check.py` (one new
  `measure_capability_adoption()` called from `validate_project()` after
  `validate_capabilities()`, plus four new finding codes in the existing
  severity vocabulary), and the new governance-root file
  `capabilities.json`. `scripts/auto_gate.py` is **not** modified — it
  already fails on governance ERRORs and writes the full finding list to its
  report, so a `required`-posture finding fails the unattended gate with no
  new code path.
- **No change to** `projects.json`, `vocabulary.json`, `decisions.json`,
  `schemas/project.schema.json`'s existing required keys, any `*_CODE`
  constant, any exit code, or the Forge adapter's observation keys.
- **No file outside this repository is written.** The audit reads consumer
  manifests and writes nothing.

## 2. The ruling

**Non-consumption of a registered capability is a measured finding, not
invisible debt.**

Three properties make it a real gate rather than a report:

1. It is **derived**, not declared. A repo cannot mark itself compliant.
2. It **escalates** on a recorded schedule, not on a human's judgement.
3. It has a **lawful exit** other than adoption: a decline with a rationale.

## 3. Why the block must change shape

The current `capabilities` entry requires `owner` and `evidence_state`
(`schemas/project.schema.json` `$defs.capability`). That is a *provider*
record. Both live users confirm it: `forge` maps `identity`/`admin`/`jobs`/
`storage`/`release` to `owner: platform` with `evidence_ref` pointing into its
own `src/`, and this repository maps `registry`/`policy`/`evidence`/`release`
the same way. A consumer has no field to write an adoption into, and
`validate_capabilities` has nothing to compare a consumer against.

The extension is additive — a new `consumes` array on each capability entry:

```json
"capabilities": {
  "idempotency": {
    "owner": "platform",
    "evidence_state": "blocked",
    "consumes": [
      {
        "provider": "dotnet-platform-libs",
        "package": "Platform.Idempotency",
        "version": "0.1.0",
        "evidence_ref": "Directory.Packages.props"
      }
    ]
  }
}
```

`consumes` is optional. A declaration without it remains valid and emits no
adoption finding, exactly as the 79 declarations that omit the block do
today. When `consumes` is present, `evidence_ref` is required per entry
(`CAPABILITY_ADOPTION_UNEVIDENCED` otherwise) and must resolve inside the
consumer root (`CAPABILITY_ADOPTION_EVIDENCE_STALE` when it stops resolving,
reusing the existing `CAPABILITY_STALE` semantics for a `verified` provider).

## 4. How adoption is measured

Measurement is a **three-way join**, and all three sides must agree:

1. **The registry** (`capabilities.json`) says the capability exists, names
   exactly one provider, and assigns a posture.
2. **The consumer's manifests** contain a pinned dependency on the named
   package. This is the only trusted side — it is what the build actually
   restores.
3. **The consumer's `.project.json`** records the pairing in `consumes[]`.

| Side present | Result | Finding |
|---|---|---|
| registry + manifest + `consumes[]` | adopted | none |
| registry + manifest, no `consumes[]` | adopted, undocumented | `CAPABILITY_ADOPTION_UNEVIDENCED` (WARNING) |
| registry + `consumes[]`, no manifest edge | declared, not wired | `CAPABILITY_ADOPTION_MISSING` |
| registry only | not consumed | `CAPABILITY_ADOPTION_MISSING` |
| local implementation + registry, no manifest edge | duplicated | `CAPABILITY_ADOPTION_MISSING`, message names the local path |

**Manifest sources, read-only and bounded:** `Directory.Packages.props`
(`PackageVersion`/`PackageReference`), `*.csproj` (`PackageReference`),
`package.json` (`dependencies`/`devDependencies`), `Cargo.toml`
(`[dependencies]`), `pyproject.toml`, `requirements*.txt`. Only the
dependency stanza is parsed. Lockfiles, transitive graphs, and registry
queries are never read: a resolution requires a network or a package-manager
invocation, both of which the audit is forbidden to perform.

**Local implementation detection** is a bounded lexical scan restricted to
declaration-supplied paths (`consumes[].local_implementation`, or a
capability entry in `capabilities.json`). This is the one heuristic in the
design, so it is deliberately narrow: the check reports *a declaration
claims a local implementation and the manifest proves no consumption* — it
does not attempt to recognise every hand-rolled retry loop. A heuristic that
guessed would produce false FAILs on the 28 rate-limiting repos, and a false
FAIL is worse than the invisible debt it replaces.

## 5. The WARN→FAIL ramp

Postures live in `capabilities.json`, one entry per capability id:

```json
"idempotency": {
  "provider": "dotnet-platform-libs",
  "package": "Platform.Idempotency",
  "posture": "expected",
  "expected_since": "2026-10-02",
  "audit_count": 0
}
```

`posture` is one of:

- `exempt` — no finding. Requires a recorded reason.
- `expected` — findings are WARNING. A new capability always starts here.
- `required` — findings are ERROR and fail `auto_gate.py`.

**Promotion to `required` requires all three, and no wall-clock shortcut:**

1. the provider's capability is `evidence_state: verified` with a resolving
   `evidence_ref` — there is nothing to consume until it exists;
2. the capability has been `expected` across `graduation_audits` consecutive
   `auto_gate.py` runs (default 3), each of which wrote a report;
3. the graduation is **appended as a ledger entry** to `capabilities.json`
   recording `from`, `to`, `audit_count`, and the evidence that satisfied (1).

The count is advanced by the auditor, but the transition to `required` is a
recorded entry — the same tamper-evident, append-only shape as
`deployment/promotions.json`, and the reason `wall_clock_days` was rejected:
an audit that has not run is not evidence that a repo refused to adopt.

`exempt` is the third state and it is the pressure valve. A capability the
provider has not published is `exempt` with reason `provider-unavailable`;
that is a governance problem with the *provider*, and charging 34 consumer
repos for it would be noise.

## 6. Finding codes

Declared in the existing `CAPABILITY_*` namespace, stable identifiers, in the
repo's `SCREAMING_SNAKE` style:

| Code | Level | Meaning |
|---|---|---|
| `CAPABILITY_ADOPTION_MISSING` | WARN (`expected`) / ERROR (`required`) | No consumption edge for a capability the registry binds to a provider. Message names the provider, the package, and any declared local implementation. |
| `CAPABILITY_ADOPTION_UNEVIDENCED` | WARNING | The manifest proves consumption but `consumes[]` is missing or has no `evidence_ref`. The edge is real; the record is not. |
| `CAPABILITY_ADOPTION_EVIDENCE_STALE` | WARNING | A `consumes[].evidence_ref` no longer resolves inside the consumer root. |
| `CAPABILITY_ADOPTION_DECLINE_UNRECORDED` | WARNING | A repo's own documentation declines or defers adoption, but no `declines[]` record exists. Names the document. |
| `CAPABILITY_ADOPTION_UNRESOLVED` | REVIEW_REQUIRED | The capability id does not bind to exactly one provider in `capabilities.json`. Never escalates, never FAILs. |

`CAPABILITY_ADOPTION_UNRESOLVED` is the deliberate fail-*soft* branch. Three
incompatible definitions of "capability" exist today, and this change does not
merge them. A finding that cannot be attributed to one provider is reported
as unresolved and is not the consumer's fault to fix — escalating it would
charge product repos for a `platform-contracts`/hermora vocabulary gap.

## 7. False positives and legitimate exceptions

A repo may legitimately not adopt. Silence is not the only acceptable answer,
but a decline record is the only one the gate accepts.

The `declines` vocabulary is closed and every member is grounded in a
boundary argument already documented in the workspace — none is a free-text
escape hatch:

| Rationale | Boundary | Grounded in |
|---|---|---|
| `ownership` | The provider's owner will not accept the responsibility | — |
| `privacy` | Adoption would move data across a declared boundary | — |
| `lifecycle` | The two projects ship and deprecate on incompatible schedules | — |
| `dependency-direction` | Adopting would invert the dependency direction | `therapist-commons/.ai-rules/workflow.md:22-25` — *"keep the capability local when ownership, privacy, lifecycle, or dependency direction makes that the correct boundary"* |
| `naming-symmetry` | Extraction is motivated by symmetry, not by a consumer boundary | `openTeacherKit/README.md:36-39` — *"Shared code belongs in the repository's protocol/model packages only after a consumer and compatibility boundary are evidenced; do not extract for naming symmetry"* |
| `not-applicable` | The capability does not apply to this repo's role | `openTeacherKit/README.md` (no network/Hub/cloud/AI requirement) |
| `provider-unavailable` | No published, verified provider exists | — |

A decline record is:

```json
"declines": [
  {
    "capability": "idempotency",
    "rationale": "dependency-direction",
    "evidence_ref": "docs/shared-ownership.md",
    "review_by": "2026-12-01"
  }
]
```

`evidence_ref` is **required** and must resolve inside the consumer root. A
decline with no resolvable document is `CAPABILITY_ADOPTION_DECLINE_UNRECORDED`
— the audit does not accept an assertion as a boundary argument, because that
is the same failure mode as a hand-declared adoption. `review_by` is advisory
and never auto-expires a decline; a stale decline is a governance conversation,
not a machine decision.

A decline is **repo-local and capability-scoped**. It never suppresses a
finding for any other capability, and it never suppresses a finding in any
other repo. That containment is deliberate: a fleet-wide "this is a platform
problem" exemption is how the 39% taxonomy disagreement became invisible in
the first place.

### Boundary scenarios

- **The provider is the consumer.** `dotnet-platform-libs` declares
  `idempotency` with `owner: platform`; it is the provider, so
  `consumes[]` is empty and the capability is `exempt` for it. A registry
  entry whose `provider` equals the audited repo is not a self-adoption
  failure.
- **Adoption is documented but the record is missing.** Five repos already
  pin `Platform.*` in `Directory.Packages.props`; they get
  `CAPABILITY_ADOPTION_UNEVIDENCED` (WARNING), never
  `CAPABILITY_ADOPTION_MISSING`. The build already restored those packages.
- **A repo declines in prose only.** `openTeacherKit` states the rule in its
  README. With no `declines[]` record, the repo gets
  `CAPABILITY_ADOPTION_DECLINE_UNRECORDED` naming the document — the audit
  found real evidence and is asking for it in the schema, not accusing the
  repo of duplicating.
- **The registry is missing.** `capabilities.json` absent or unparseable is
  a governance ERROR. There is no empty-registry fallback, matching the
  `vocabulary.json` rule: a check that degrades to "nothing to enforce"
  would silently restore today's invisibility.
- **The capability id is ambiguous.** Three vocabularies disagree, so
  `idempotency` may bind to two providers. `CAPABILITY_ADOPTION_UNRESOLVED`
  at REVIEW_REQUIRED, no FAIL, no repo is charged.
- **A repo is not in the registry at all.** `argoscope`, `chronicleaf`,
  `devloom`, and `kairovia` have `.project.json` and no entry. Adoption
  measurement is registry-driven, so they are out of scope here;
  `DISCOVERED_UNREGISTERED` remains their finding. The eight entries whose
  `id` differs from their directory name (`campusMesh` vs `campus-mesh`)
  resolve through `path` and are not affected — this change matches on
  `provider` id and never on a directory name.
- **The audit runs with no project command.** Consumption is proven by a
  pinned coordinate, so the audit proves a *declaration of intent to
  restore*, not that the restore succeeded. `evidence_state` and the
  existing invariant stand: a declaration is intent, not evidence.

## 8. Compatibility and non-interference

- 79 declarations without `consumes[]` emit no adoption finding.
- 0 new ERRORs on a clean fleet: every capability starts `expected`, so
  every new finding is a WARNING.
- `maturity_audit.py` still reports the same single result
  (`workspace_check.py` output) and never computes a second opinion.
- The Forge adapter is untouched: new findings appear in the existing
  `findings` list, and no observation key is added, removed, or renamed.
- Exit codes, `--json` shape, and the `auto-gate-report.json` schema are
  unchanged; new findings ride the existing list.

## 9. Verification oracle

Schema fixtures for `consumes[]` (valid, missing `evidence_ref`, escaping
`evidence_ref`, non-object entry) and for `declines[]` (valid, unknown
rationale, unresolvable `evidence_ref`). Portfolio fixtures reproducing the
34-repo idempotency and 28-repo rate-limiting splits. One decline fixture per
rationale class, including the two grounded in existing repository prose. A
read-only proof: recursive tree hash of a fixture project is identical before
and after the audit, and no child process is spawned. A ramp test asserting a
capability cannot reach `required` without a verified provider and the
recorded graduation entry. A test asserting a decline in one repo never
suppresses a finding in another.

## 10. Decision ledger

Resolved: measurement joins registry + consumer manifest + `consumes[]`, and
all three must agree; the manifest is the only trusted side; `required`
requires a verified provider, a consecutive-audit count, and a recorded
graduation entry; declines are closed-vocabulary, evidence-backed,
repo-local, and capability-scoped; the finding namespace is
`CAPABILITY_ADOPTION_*`; an unresolvable capability id is REVIEW_REQUIRED
rather than FAIL.

Deferred: merging the three capability vocabularies
(`platform-contracts` contract family, the declarative block, hermora's
endpoint list) into one id space — a `platform-contracts` change, and the
reason `CAPABILITY_ADOPTION_UNRESOLVED` exists. Behavioural proof of
adoption (a registered-shared-wrapper test signal) rather than a pinned
coordinate. Rewriting `kind`, which conflates role and stack. The `tests:
true` gate-command binding, owned by the `driftwatchdog` repository and
referenced, not duplicated.

Blockers: none for the measurement. The remediation path — the permission to
actually repoint a consumer — is owned by `cross-repo-adoption-grant`. This
change is valid and useful without it: the gate measures, records, and
reports, and the absent grant shows up as a decline or an open finding rather
than as hidden work.
