# Tasks: platform-privacy

## 0. Conventions

- [x] **0.1** Mirror `Platform.Quota` layout, central package versions, nullable + warnings-as-errors, XML docs on public APIs.
- [x] **0.2** Architecture tests: `Platform.Privacy` references only `Platform.Core`; AspNetCore adapter only Core + Privacy. Three new facts in `DependencyDirectionTests`.

## 1. Core contracts

- [x] **1.1** `ConsentCategory`, `ConsentRecord`, `IConsentStore`.
- [x] **1.2** `ISubjectExportProvider`, `ISubjectErasureProvider`, registry, `ExportBundle`, `ErasureReport`.
- [x] **1.3** `ConsentPolicy` evaluation: fail-closed, version-gated re-consent.
- [x] **1.4** `InMemoryConsentStore`.
- [x] **1.5** `EmailMask`, `StablePseudonym`.

## 2. AspNetCore adapter

- [x] **2.1** `ConsentCookieService` + options (name, lifetime, version); unparsable falls back to Necessary-only.
- [x] **2.2** Scoped consent state resolved per request.
- [x] **2.3** `RequireConsentAttribute` + enforcement middleware (403 problem details naming missing categories).
- [x] **2.4** Explicit anonymous-subject policy (`UnresolvedSubjectBehavior`, default Deny); no silent shared bucket.
- [x] **2.5** DI extensions for both packages.

## 3. Solution wiring

- [x] **3.1** Both projects + both test projects in `Platform.sln` (`dotnet sln add`).
- [ ] **3.2** `docs/packages.md` entries (done); public-API baseline update if the repo requires it.

## 4. Verification

- [x] **4.1** New tests: 9 core + 4 adapter, all passing
      standalone (13 total). Full `Platform.sln`: all green except 3
      pre-existing failures from the subagent's
      `ui-shared-layer-reachability` archive (tests still read the
      active change path; untouched by this change, recorded in
      HANDOFF). Revert rows (build-verified, restored): stale
      version grants → 1 fail; swallowed provider failure → 1 fail;
      unenforced endpoint → 1 fail. `Only_..._FrameworkReference`
      allowlist extended for the new adapter.
- [x] **4.2** `openspec validate --all --strict`: 53/53.
- [ ] **4.3** `scripts/conformance.sh` (running in background),
      `git diff --check` clean.
