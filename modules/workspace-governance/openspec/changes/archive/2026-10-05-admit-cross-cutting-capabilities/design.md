# Design: Admit cross-cutting shared capabilities into the profile vocabulary

## 1. Implementation boundary

Repository `/home/paul/code/workspace-governance`, Python 3 standard library
only. No new dependency and no new runtime.

- **Extension points are exactly:** the twelve `profiles/*.json` documents that
  already declare a `supported_capabilities` list, the vocabulary paragraph in
  `docs/capabilities.md`, and one new test module. `scripts/workspace_check.py`
  is **not** modified: `validate_capabilities` and
  `validate_project`'s `load_profile_spec` read are unchanged.
- **No change to** `capabilities.json`, `projects.json`, `vocabulary.json`,
  any `*_CODE` constant, any exit code, the schema, or the Forge adapter.
- **No file outside this repository is written.** Consuming the corrected
  vocabulary is a separate, bounded authorization in the consumer.

## 2. The defect is a data mismatch, not a missing check

`validate_capabilities` reports WARNING `CAPABILITY_UNSUPPORTED` when a
declared capability id is not in the profile's `supported_capabilities` list.
That check is sound: it keeps the supported set intentional.

The mismatch is that `capability-adoption-gate` introduced `consumes[]` for
**registry** capability ids (`telemetry`, `rate-limiting`, `idempotency`,
`feature-flags`, …) while the profiles were written before the registry
existed and enumerate only the seven product-shaped ids. Every registry id
except `identity` — which the product profiles already carried for a different
reason — is therefore unsupported in every profile. The fix is to widen the
data, not to add or remove a check.

## 3. Why these four ids and not the other four

The registry binds nine ids. Four are cross-cutting and four are role-shaped:

| Id | Property | Admitted here |
|---|---|---|
| `telemetry` | cross-cutting: every host, library, and tool emits it | yes |
| `rate-limiting` | cross-cutting: every request boundary governs it | yes |
| `idempotency` | cross-cutting: every at-least-once consumer needs it | yes |
| `feature-flags` | cross-cutting: every deployable surface gates behavior | yes |
| `identity` | already in every product profile | already |
| `persistence-postgres` | a persistence role | on demand |
| `health-checks` | a hosting role | on demand |
| `web-application-factory` | a test-harness role | on demand |
| `postgres-test-fixture` | a test-harness role | on demand |

A profile list is a maintainer's intentional statement, so it is widened for
the class that applies everywhere and left to grow for the ids that apply to a
specific role. That boundary is stated in the proposal's non-goals so a later
reader is not left wondering why half the registry is absent.

## 4. The rejected alternative: union the profile list with the registry

The competing fix is to compute the supported set in `validate_project` as the
union of the profile's list and every id in `capabilities.json`. It is more
automatically complete, but it is rejected for three reasons:

1. **It makes the profile list redundant.** If the registry subsumes the list
   for every shared id, then the list only governs the product-shaped ids, and
   the profile stops being the place that states intent for shared capabilities.
2. **It changes the meaning of a hook that reads no registry today.**
   `load_profile_spec` is documented as reading the profile "to surface a soft
   warning"; the union makes that read conditional on a second file and a
   second failure mode the warning was never about.
3. **It removes a review step.** Admitting a registered id to a profile is a
   one-line edit a maintainer makes deliberately; a union admits a new
   registry id to every profile the moment the registry changes, with no
   per-profile decision and no diff to review.

The data edit is the smaller blast radius and keeps the check's purpose.

## 5. Verification oracle

- A test asserts every profile that declares a `supported_capabilities` list
  contains all four cross-cutting ids. It fails naming the profile and the
  missing id, so the rule cannot drift back.
- A test calls `validate_capabilities` with a `dotnet-product` supported set
  and a `telemetry` declaration and asserts **zero** findings — the measured
  reproduction from the proposal, now green.
- A **negative control** calls it with `dotnet-library` and a capability the
  profile does not list, and asserts `CAPABILITY_UNSUPPORTED` is still emitted.
  A widening that also silenced the genuine warning would turn this red.
- `python3 -m unittest discover -s tests` stays green, `--self-check` stays
  `0 errors / 0 warnings`, and `openspec validate --all --strict` is unchanged
  apart from the new change itself.

## 6. Decision ledger

Resolved: the fix is a profile-data change; the admitted set is the four
cross-cutting shared ids; the registry's role-shaped ids are admitted on
demand; the checker is unchanged; the negative control is preserved.

Deferred: making `supported_capabilities` registry-aware, and uniformly
admitting `persistence-postgres`, `health-checks`, `web-application-factory`,
and `postgres-test-fixture` — both are separate decisions the first consumer of
those ids should argue.

Blockers: none.
