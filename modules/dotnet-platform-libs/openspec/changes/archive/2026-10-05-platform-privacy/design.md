# Design: platform-privacy

## 1. Package shape (mirrors Quota)

```
src/Platform.Privacy/               # no ASP.NET Core, no EF Core (guarded)
  Contracts/                        # ConsentCategory, ConsentRecord, IConsentStore,
                                    # ISubjectExportProvider, ISubjectErasureProvider,
                                    # ExportBundle, ErasureReport
  Evaluation/                       # ConsentPolicy (version + evaluation, fail-closed)
  Anonymization/                    # EmailMask, StablePseudonym (HMAC-SHA256, app key)
  Stores/                           # InMemoryConsentStore
  DependencyInjection/              # ServiceCollectionExtensions
src/Platform.Privacy.AspNetCore/    # may depend on Core + AspNetCore only
  Cookies/                          # ConsentCookieService (read/write, options: name, lifetime, version)
  Enforcement/                      # RequireConsentAttribute, ConsentEnforcementMiddleware (403 + problem details)
  DependencyInjection/              # ServiceCollectionExtensions
tests/Platform.Privacy.Tests/
tests/Platform.Privacy.AspNetCore.Tests/   # TestServer via Platform.Testing.AspNetCore patterns
```

## 2. Decisions

- **Enum categories, not strings.** Four fixed values; applications needing more model them as sub-purposes in their own data, not in shared vocabulary.
- **Version-gated re-consent.** The record carries the policy version it was granted under; evaluation denies optional categories on mismatch. Bumping the version is the application's one-line re-consent mechanism.
- **Fail closed everywhere.** Missing record, unparsable cookie, unknown subject, provider throw: deny / Necessary-only / recorded failure. No silent shared-subject bucket (same rule as quota's missing-context policy).
- **Cookie is plain JSON, not signed.** DataProtection wiring differs per host and would couple the package to hosting decisions; the cookie carries grants, not secrets, and tampering degrades to Necessary-only. Documented, not hidden.
- **No endpoints, no banner.** Routes and markup are product surface; the adapter ships metadata + middleware + state, exactly like quota ships enforcement without plans.
- **Pseudonym keys stay out.** `StablePseudonym` takes the key per call; storage and rotation are application-owned.

## 3. Risks

| risk | answer |
|---|---|
| category enum too narrow | fixed four covers GDPR practice; wider taxonomies live in app data |
| unsigned cookie replay | grants are non-sensitive; worst case is a subject granting themselves *fewer* rights (deny) or re-granting analytics — accepted, documented |
| erasure vs legal retention | providers decide per subject; registry reports, never overrides |

## 4. Not done, and named

- DataProtection-signed consent cookies (host-coupled; revisit on consumer evidence).
- Retention-policy scheduling (needs a jobs story; Platform.Jobs exists — a later change wires it).
- Banner UI in Platform.UI.Razor (markup; needs a consumer first — openlearning's banner is the candidate).
