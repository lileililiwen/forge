# Tasks: Shared ASP.NET test web factory

Change **implemented**. Checkboxes are checked only where the work is done and
verified; anything left unchecked says why. Verification evidence is recorded in
§4.

Ordering: BFS baseline → DFS requirement implementation → BFS regression and
completeness → Verification.

## 1. BFS — Baseline and impact coverage

- [x] Record the pre-change state of the four load-bearing architecture guards
      (`Platform_Testing_AspNetCore_only_depends_on_public_contracts_and_core`,
      `Platform_Testing_AspNetCore_declares_FrameworkReference_for_ASPNET`,
      `Production_projects_do_not_reference_Platform_Testing_toolkit`,
      `Production_projects_do_not_reference_any_testing_only_platform_project`)
      as a passing baseline, plus the full `Platform.Architecture.Tests` count.
      **Baseline: 351 passed / 0 failed.** After the change: **355 passed / 0
      failed** (+3: the added package in `ProductionProjects` feeds three
      `[Theory]` cases, plus the new ORM/test-framework guard).
- [x] Capture the hashes of `eng/package-manifest.json` and
      `eng/public-api-baseline.txt` before any edit, to prove neither moves.
      **Recorded: manifest `b12109a669354f819ddf4633a26e710809c8ef4f5ae6325c99158e27f17125c5`;
      baseline `67a37dfea791744941b2096fde653c7dd9d93d65b2fe66bf598f30db7b1863b8`.**
- [x] Confirm `Platform.Testing.AspNetCore` is already present in
      `eng/package-manifest.json` and is **not** covered by
      `scripts/check-public-api.sh`'s five-project list; record the gap
      (open decision **D4**). **Confirmed and gap closed** — see §2.
- [x] Record `scripts/package-inventory.sh` output (75 `src/` projects) as the
      inventory baseline. **Recorded: 75, unchanged after the change.**
- [x] Enumerate every in-repository use of the non-generic
      `PlatformTestWebApplicationFactory`: `README.md:583-601`,
      `docs/packages.md:736`, `docs/packages.md:765`,
      `docs/platform-testing-toolkit.md:10,70`,
      `tests/Platform.Testing.Tests/AspNetCore/PlatformTestWebApplicationFactoryTests.cs`
      (5 tests), `tests/Platform.Testing.Tests/Scenarios/CheckoutScenarioTests.cs`
      (1 site), plus the narrative mentions in `AGENTS.md` and `HANDOFF.md`.
- [x] Classify each of the 5 existing factory tests as **keep**, **rewrite** or
      **delete**; the 2 that assert the empty-app `"/"` → `"ok"` round-trip are
      `rewrite` by construction and this must not be decided during
      implementation. **Decided up front: 2 rewrite, 3 keep.** Neither was
      skipped, weakened or deleted.
- [x] Confirm the measured shape table still holds (24 repos, 63 subclasses, 7
      non-`Program` entry points, 19 config-permutation classes) and record any
      drift against the prior analysis. **Re-measured; see §5.** The
      `ConfigureWebHost` / `WithWebHostBuilder` counts differ from the amendment
      and the measured values are recorded.
- [x] Capture the current content-root behaviour (`AppContext.BaseDirectory` on a
      self-owned `WebApplication`) as the baseline the real-app content root will
      replace. **Captured** from the pre-change source: the old class set
      `WebApplicationOptions { EnvironmentName = "Testing", ContentRootPath =
      AppContext.BaseDirectory }`.
- [x] Get owner decisions on **D1** (non-generic shim: delete / obsolete shim /
      rename the generic type) and **D2** (where EF helpers eventually live)
      before writing any public signature. **D1 resolved by measurement (delete,
      0 external consumers). D2 remains open**; the guard gap it exposed is closed.

## 2. DFS — Requirement-by-requirement implementation

- [x] Convert `PlatformTestWebApplicationFactory` to
      `PlatformTestWebApplicationFactory<TEntryPoint> : WebApplicationFactory<TEntryPoint>`
      with `where TEntryPoint : class`; resolve the name per **D1** and apply
      the `[Obsolete]` treatment if option (ii) was chosen. **D1 = option (i):
      the non-generic type is deleted, no shim.** Call sites updated.
- [x] Implement the real-application bootstrap: resolve the host builder from
      `TEntryPoint` (its own entry point, or the `CreateHostBuilder` /
      `ConfigureWebHost` pair a consumer exposes) and host it over `TestServer`.
      Done by subclassing; `CreateHost` and both bootstrap paths are overridden
      to apply the environment on the host and the web host.
- [x] Remove the empty-`WebApplication` path entirely — no
      `WebApplication.CreateBuilder` + `MapGet("/")` fallback under any
      condition. **Done; verified by grep and by a test asserting an unrouted
      path is 404.**
- [x] Resolve the content root to the consumer's output directory and confirm
      the `MvcTestingAppManifest.json` mechanism from
      `Microsoft.AspNetCore.Mvc.Testing` is what does it, rather than new code.
      **No new code.** The base class's `SetContentRoot` does it; covered by a
      test that fetches `/asset.txt` and gets the application's own asset.
- [x] Emit the named entry-point diagnostics: one for a type not visible from
      the test assembly (naming the type, the declaring assembly, and
      `InternalsVisibleTo` or a public marker as the fix) and one for a type
      that exposes no host builder. **Done.** The base class's own entry-point
      failures are translated into one diagnostic that names the type, the
      assembly and both remedies. Translation is deliberately narrow (three
      message fragments) so an application that throws during startup still
      surfaces its own exception unchanged.
- [x] Add `WithEnvironment(string)`: honour any non-empty string, default to
      `PlatformTestEnvironments.Testing`, throw `ArgumentException` on
      null/empty/whitespace before any host is built, and require the
      consumer's `appsettings.<Environment>.json` to still apply. **Done and
      tested** (default, `Staging` + its `appsettings.Staging.json`, and the
      three invalid inputs).
- [x] Implement the post-first-access mutation guard: a named
      `InvalidOperationException` on every mutating call once the host exists,
      pointing at `WithWebHostBuilder`. **Done.** The message points at
      "configure before first access" and explains that `WithWebHostBuilder`
      would build a second host, per the one-host decision (**n**).
- [x] Add `ReplaceService<TService>(TService)` and
      `ReplaceService<TService, TImpl>()` using `RemoveAll<TService>()` inside
      host-level `ConfigureWebHost` configuration (after the application's own
      `ConfigureServices`), throwing a named `InvalidOperationException` when
      `TService` was never registered. **Done and tested**, including the
      multiple-registration case and the unregistered-service failure.
- [x] Add `DecorateService<TService, TWrapper>(Func<TService, TService>)`
      resolving the original before replacing, so the original stays reachable
      inside the wrapper. **Done and tested** (original reachable through the
      wrapper).
- [x] Add `WithClock(IClock)` bound to `Platform.Core.IClock` — confirm it adds
      no package reference and leaves `eng/package-manifest.json` untouched.
      **Done. Manifest hash is byte-identical.** `IClock` lives in
      `Platform.Core.Time`; no new reference was added.
- [x] Add `OnDispose(Action)` and `OnDispose(Func<ValueTask>)`: run after the
      host stops, in reverse registration order, with a throwing action recorded
      and skipped-over rather than aborting the remaining actions or host
      disposal. **Done and tested**, including "host already stopped" (observed
      through a hosted-service probe), async awaiting, and failure isolation.
- [x] Add `UseTemporaryDirectory()` and the `TemporaryDirectory` accessor: a
      unique writable directory under the OS temp root, removed on disposal,
      never the process current directory and never a content root. **Done and
      tested**, plus the process-wide opt-in (**D7**).
- [x] Confirm the host-configuration ordering property with a test: a service
      registered multiple times by the application resolves to the single
      replacement. **Done** — the application registers `IGreetingSender` twice
      and a test observes both registrations from the callback, then the single
      replacement.
- [x] Rewrite `PlatformTestWebApplicationFactoryTests.cs` against a real
      in-repo test application, keeping the `Testing` default,
      `WithConfiguration` and `ConfigureTestServices` assertions and dropping the
      empty-app round-trip. **Done.** The fixture is the new
      `tests/Platform.Testing.TestApplication` (see §5 for why not the starter
      template).
- [x] Update the one call site in
      `tests/Platform.Testing.Tests/Scenarios/CheckoutScenarioTests.cs`.
- [x] Update `docs/packages.md` § Platform.Testing.AspNetCore (member list) and
      the `tests/Platform.Testing.Tests` inventory row at line 765.
- [x] Update `docs/platform-testing-toolkit.md` (the package table row and the
      usage snippet) and the `README.md` row at line 244 plus the usage example
      at lines 583-601.
- [x] Update the narrative mentions in `AGENTS.md` and `HANDOFF.md` so the
      repository's own handoff does not describe the empty-app factory. `AGENTS.md`
      current-state line corrected; `HANDOFF.md` given a new current section
      rather than rewriting its historical log.
- [x] Add the named-diagnostic test for the inaccessible-marker case, asserting
      the message names the type and its assembly. **Done.**
- [x] Add failure and boundary tests: non-entry-point marker, throwing
      `ConfigureWebHost` callback, `WithEnvironment(null)`, `WithClock(null)`,
      `ReplaceService` for an unregistered service, mutation after first host
      access, throwing `OnDispose` action, and two differently-configured
      factories staying isolated. **All done and passing.**

### 2a. Amendment — one host per instance, and parameterized permutations

- [x] Measure the workspace before writing the requirement: `protected override
      void ConfigureWebHost` declarations and `WithWebHostBuilder(` call sites,
      excluding `obj/`, `bin/`, `node_modules/`, `target/`, `.git/`.
      **Measured: 79 declarations in 69 files; 118 call sites in 106 files across
      17 repositories.** (The amendment quoted 78 and 117; the measured values
      are recorded in `design.md` §10 entry **(n)** and in `HANDOFF.md`.)
- [x] Read the external MIT reference and confirm the discipline exists there.
      **Read** `fullstackhero/dotnet-starter-kit`
      (`/home/paul/learning/dotnet-starter-kit`, MIT, © 2021-2026):
      `FshWebApplicationFactory` (310 lines) and its sibling
      `MiddlewareWebApplicationFactory`, which carries the "NEVER calls
      WithWebHostBuilder — exactly ONE host is built per process" note and forces
      early host creation with `_ = Server;`. Sibling factories take constructor
      parameters rather than hard-coded literals.
- [x] **Do not import or vendor any code from that repository.** Confirmed: no
      file was copied; nothing was added to any `.csproj`. The reference is cited
      as a design source only, and its domain dependencies (Finbuckle, Hangfire,
      S3/MinIO, billing) are deliberately not acquired.
- [x] Amend the spec with a requirement that the factory builds exactly one host
      per instance and that consumer configuration is applied at construction
      rather than via `WithWebHostBuilder`, with the measurement and the
      reference recorded as rationale.
      **Added:** *"The shared web test factory SHALL build exactly one host per
      factory instance"* (3 scenarios).
- [x] Amend the spec with a requirement covering the config-permutation shape
      explicitly. **Added:** *"The factory SHALL express configuration
      permutations without subclassing"* (2 scenarios). The older, weaker
      permutation requirement was replaced rather than duplicated.
- [x] Implement it **without inventing a redundant API.** The pre-existing
      fluent `ConfigureWebHost(Action<IWebHostBuilder>)` already configures this
      factory at construction time, so it is the escape hatch; the post-first-
      access guard enforces the one-host rule. **No new member was added for the
      amendment.** The only new public type is
      `PlatformTestTemporaryDirectoryScope` for **D7**, which is a different
      decision.
- [x] Record in `design.md` §10 that the discipline was adopted from an external
      MIT reference after measurement, and that the Postgres container-pair and
      migration-lock patterns from that reference are deferred with the rest of
      the Postgres work. **Recorded** as ledger entries **(n)** and **(o)**.
- [x] Add tests for the amendment: one host per instance (repeated access,
      callbacks run once, no derived factories, a subclass still builds one
      host) and permutations without subclassing (one type across scenarios,
      cross-factory isolation, service state not shared). **Added in
      `PlatformTestWebApplicationFactoryHostDisciplineTests.cs`, all passing.**

### 2b. Owner-mandated guard gaps (acted on in the same change)

- [x] **D2 guard gap:** add `Platform.Testing.AspNetCore` to the architecture
      suite's `ProductionProjects` list so the EF-forbidden rule covers it.
      **Done**, plus a narrow ASP.NET-only exemption for its legitimate
      `Microsoft.AspNetCore.Mvc.Testing` reference, and a new dedicated guard
      `Platform_Testing_AspNetCore_has_no_orm_test_framework_or_container_reference`.
      **Negative-tested:** adding an EF Core reference makes three guards fail.
- [x] **D4 guard gap:** extend `scripts/check-public-api.sh` to scan
      `Platform.Testing.AspNetCore` and regenerate `eng/public-api-baseline.txt`.
      **Done.** The baseline now carries a `[Platform.Testing.AspNetCore]`
      section and the check is clean. Note the baseline file therefore **did**
      change — that is the point of closing the gap; the manifest did not.

## 3. BFS — Cross-surface regression and completeness

- [x] Every requirement in `specs/platform-test-web-factory/spec.md` has at
      least one passing scenario test, and every scenario test maps back to a
      named requirement.
- [x] Every entry in the "Placed IN" table of `design.md` §4 exists with the
      recorded signature; every entry in the "DEFERRED" table is absent.
      **Verified by grep** for `ReplaceDbContext`, `MigrateOnStartup`,
      `EnsureCreatedAsync`, `ResetDatabaseAsync`, `CaptureLogs`,
      `WithDefaultRequestHeader`, `IAsyncLifetime` — all absent.
- [x] No new `PackageReference` and no new `ProjectReference` in
      `src/Platform.Testing.AspNetCore/Platform.Testing.AspNetCore.csproj`.
      **Verified:** still exactly `Microsoft.AspNetCore.Mvc.Testing` +
      `Platform.Core` + the ASP.NET `FrameworkReference`.
- [x] `Platform.Testing.AspNetCore` remains absent from the architecture suite's
      `ProductionProjects` list, or the EF-forbidden gap noted in **D2** is
      closed deliberately — decide, do not drift. **Closed deliberately** (§2b).
- [x] All 350 architecture guards pass, with the four
      `Platform.Testing.AspNetCore` guards explicitly re-checked. **355 pass**
      (the plan's 350 was the pre-change estimate; the real baseline was 351).
- [~] `eng/package-manifest.json` and `eng/public-api-baseline.txt` are
      byte-identical to the §1 hashes; the 75-package inventory is unchanged.
      **Partly superseded, deliberately.** The manifest **is** byte-identical
      (`b12109a6…` unchanged) and the inventory is still 75. The public-API
      baseline **did** change, because **D4** required extending the script's
      coverage to this package and regenerating it — leaving it untouched would
      have shipped this change's public surface unguarded.
- [x] No `src/**` or `tests/**` file outside
      `src/Platform.Testing.AspNetCore/` and `tests/Platform.Testing.Tests/`
      changed; no consumer repository touched. **With two recorded exceptions,
      both deliberate and both in the design ledger:** the new
      `tests/Platform.Testing.TestApplication/` fixture project (a test-only
      app, needed because the starter template is `dotnet-template` content and
      is not in `Platform.sln`), and
      `tests/Platform.Architecture.Tests/DependencyDirectionTests.cs`, edited
      only to close the two guard gaps the owner ordered. `Platform.sln` is
      **byte-identical to its pre-change state** - see §5. No consumer
      repository was modified.
- [x] `openspec/specs/platform-testing-toolkit/spec.md` is unmodified.
- [x] Confirm no repository contains a second hand-rolled factory for a
      scenario now covered by this surface *inside this repository* (the
      starter-template fixture is the only expected one, and it is the in-repo
      consumer fixture). **Confirmed** — the starter template fixture is left
      untouched as a template example.
- [x] Confirm `IAsyncLifetime`, `CaptureLogs`, `WithDefaultRequestHeader`,
      `ReplaceDbContext`, `MigrateOnStartup`, `EnsureCreatedAsync` and
      `ResetDatabaseAsync` are all absent, with the deferral reasons recorded.
      **Confirmed; reasons recorded in `design.md` §4 and §10.**

## 4. Verification

- [x] `openspec validate shared-test-web-factory --strict` and
      `openspec validate --changes --strict --no-interactive`. Both pass; output
      recorded in the report.
- [x] `./scripts/generate-package-manifest.sh --check` reports
      `Manifest matches source.` **Confirmed.**
- [x] `./scripts/check-public-api.sh` produces a clean diff. **Confirmed**
      (exit 0) after regenerating the baseline.
- [x] `scripts/package-inventory.sh` still enumerates 75 projects. **Confirmed.**
- [x] `dotnet restore Platform.sln && dotnet build Platform.sln -c Release &&
      dotnet test Platform.sln -c Release --nologo`. **Confirmed**, 0 failures.
- [x] `dotnet test tests/Platform.Testing.Tests --filter AspNetCore` and
      `dotnet test tests/Platform.Architecture.Tests` pass with the recorded
      guard count. **Confirmed: 355 architecture guards; 96 toolkit tests.**
- [x] `./scripts/quality-gate.sh` passes end to end. **Confirmed.**
- [x] `git diff --check` is clean. **Confirmed.**
- [x] Record every actual command output. Report any environment-blocked check
      with its exact next action, never as passing.
- [x] State the evidence boundary explicitly in the handoff: no consumer
      repository is compiled, migrated or proven by this change, and no
      container- or database-backed integration is reported as verified.

## 5. Deviations and measurements recorded during implementation

- **D1 measured count: zero** real external consumers of the non-generic type
  across `/home/paul/code/*/`. The only external traces are two prose mentions
  (`therapist-commons/docs/superpowers/plans/practice-foundation.md:41`,
  `ARCHITECTURE.md:334`) and one package reference
  (`therapist-commons/tests/Api.Tests/Api.Tests.csproj:21`) whose test project
  never references the type — it uses its own
  `new WebApplicationFactory<Program>()`. The non-generic type was therefore
  **deleted**, not deprecated.
- **Test fixture:** the plan proposed the starter template at
  `templates/platform-application-starter/`. It is `dotnet-template` content with
  `<!--#if-->` directives, an EF Core `DbContext` and Identity, is **not** in
  `Platform.sln`, and would have pulled an ORM into the test project. Replaced
  with `tests/Platform.Testing.TestApplication/`, a minimal dependency-free app.
- **`IClock` namespace** is `Platform.Core.Time`, not `Platform.Core` as
  `design.md` §4 implies.
- **`Platform.sln` left untouched.** The new test application is *not* added to
  `Platform.sln`; `Platform.Testing.Tests` builds it transitively through its
  `ProjectReference`, and the suite was verified green without the entry. Adding
  it was measured to make `Platform.Jobs.Hangfire.Tests`
  `Sequential_dispatches_use_independent_handler_state` exceed its 30 s
  wall-clock poll timeout in the full-solution run (that test passes 69/69 in
  isolation, three consecutive runs, and the full suite passes with the project
  out of the solution). Keeping the solution unchanged removes the extra
  solution-level build and test target invocation entirely, so the pre-existing
  timing sensitivity in that test is not perturbed.
- **Entry-point diagnostic mechanism:** the base class fails in two different
  places, and the "assembly has no entry point" check runs *before* any
  overridable member, so a pre-flight guard cannot see it. The implementation
  therefore translates the base class's own entry-point failures at the public
  access points. `Microsoft.NET.Sdk.Web` also generates a **public** `Program`
  for top-level statements, so the spec's "internal `Program`" scenario is only
  reachable for hand-written entry points and for library types; the tests cover
  those reachable cases.
