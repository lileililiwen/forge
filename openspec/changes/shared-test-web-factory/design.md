# Design: Shared ASP.NET test web factory

Planning artifact. **Nothing in this document is implemented by this change.**
It records the boundary, the contracts, the failure policy, the migration path
and the decisions that are still open for the owner.

## 1. Implementation boundary

> **Status: implemented.** This document was written as a planning artifact; the
> change has since been built. Where implementation and this document disagree,
> `specs/platform-test-web-factory/spec.md` wins, and ledger entries
> **(g)**–**(o)** record the decisions taken since.

Repository `/home/paul/code/dotnet-platform-libs` — C#/.NET (SDK `10.0.400`,
`net10.0`), 75 `src/` packages.

**Files this change would change when implemented:**

- `src/Platform.Testing.AspNetCore/PlatformTestWebApplicationFactory.cs` — the
  generic factory, rewritten in place.
- `src/Platform.Testing.AspNetCore/PlatformTestEnvironments` (same file today) —
  unchanged; the `Testing` constant stays.
- A non-generic compatibility shim — **not created.** **D1** was resolved by
  measurement to option (i), delete; the measured external consumer count is zero
  (ledger entry **(g)**).
- `tests/Platform.Testing.Tests/AspNetCore/PlatformTestWebApplicationFactoryTests.cs`
  — 5 existing tests; **2 assert the empty-`WebApplication` behaviour and must be
  rewritten**, not preserved.
- `tests/Platform.Testing.Tests/Scenarios/CheckoutScenarioTests.cs` — 1 call site.
- `tests/Platform.Testing.TestApplication/` — a minimal in-repo ASP.NET Core
  application (entry point, routes, a static asset, `appsettings.json` and
  `appsettings.Staging.json`) that the rewritten tests boot. The starter template
  at `templates/platform-application-starter/` is `dotnet-template` content with
  `<!--#if-->` directives, an EF Core `DbContext` and Identity, is not in
  `Platform.sln`, and would have pulled an ORM into the test project; a dedicated
  test-only application is the smaller, dependency-free fixture. It is a test
  project, is excluded from the package inventory, and adds no package.
- `docs/packages.md` (§ Platform.Testing.AspNetCore, and the
  `tests/Platform.Testing.Tests` inventory row at line 765),
  `docs/platform-testing-toolkit.md` (lines 10 and 70), `README.md` (row at line
  244, usage example at lines 583-601).
- `openspec/specs/platform-test-web-factory/spec.md` — created on archive only.

**Must not change:** any other `src/**` or `tests/**` project;
`eng/package-manifest.json`; `scripts/generate-package-manifest.sh`;
`Directory.Build.props` / `Directory.Packages.props`; `global.json`;
`openspec/specs/platform-testing-toolkit/spec.md`; and every consumer
repository.

**Deliberately changed beyond the original list**, each because the change would
otherwise be unguarded — both are defects in their own right:

- `eng/public-api-baseline.txt` and `scripts/check-public-api.sh` — the baseline
  covered only five projects and **not** this package, so this change's entire
  public surface would have shipped unguarded (**D4**). The script now scans the
  package and the baseline is regenerated.
- `tests/Platform.Architecture.Tests/DependencyDirectionTests.cs` — the
  `ProductionProjects` list omitted `Platform.Testing.AspNetCore`, so the
  EF-forbidden rule did not cover it (**D2**). The package is now listed.

`Platform.sln` is **not** changed: the new test application is built transitively
through `Platform.Testing.Tests`, so the solution stays byte-identical to its
pre-change state (see `tasks.md` §5).

## 2. Subclass, not wrapper

**Decision: `PlatformTestWebApplicationFactory<TEntryPoint> : WebApplicationFactory<TEntryPoint>`,
`where TEntryPoint : class`.**

One-line rationale: all 63 subclasses and all 117 anonymous sites are already
`WebApplicationFactory<T>`, so subclassing inherits `ConfigureWebHost`,
`CreateClient(WebApplicationFactoryClientOptions)`, `ConfigureClient`,
`WithWebHostBuilder` and `Server` for free, while a wrapper would re-implement
the `TestServer` plumbing *and* force all 117 sites to be rewritten.

The generic parameter is not decoration. C# has **no default type parameter** for
classes, so there is no way to express "defaults to `Program`" — the parameter
must be supplied, and that is what makes the 7 non-`Program` markers work:

| Type argument | Observed uses | Why it matters |
|---|---|---|
| `Program` (global namespace, top-level statements) | 409 | the common case |
| `Arivio.Api.Program`, `HistoryKB.Web.Program` | 11, 1 | `Program` inside a namespace; not the global one |
| `Stylify.Web.ProgramMarker` | 8 | explicit public marker |
| `MinimalWebSampleMarker`, `IdentitySampleMarker` | 3, 3 | in-repo sample markers |
| `SomadaNote.Web.WebAssemblyMarker`, `SomadaNote.Api.ApiAssemblyMarker` | 1, 1 | per-layer public markers |

This repo already owns the marker pattern: `src/Platform.Web/PlatformWebAssemblyMarker.cs`
is a `public sealed` marker used exactly this way. The factory does **not**
require that type and does not depend on `Platform.Web`; a consumer that has
`Platform.Web` may use it, and a consumer that does not declares its own.

## 3. Capability boundary

**New capability: `platform-test-web-factory` (ADDED Requirements only).**

`platform-testing-toolkit` is deliberately **not** modified. Its shipped
requirements — determinism, inspectable fakes, "test dependencies SHALL not leak
into production packages", "integration prerequisites SHALL be explicit" — are
all still exactly true, and none of their wording needs to change. The web
factory simply was never described there, so there is nothing to `MODIFY`. The
interaction is a *constraint we inherit*, not a contract we rewrite:

- the no-framework-dependency rule is **why** `IAsyncLifetime` is a non-goal;
- the prerequisite-honesty rule is **why** this change must not add a database
  fixture and must not let a Docker-requiring test report as passing.

## 4. Public surface (names and signatures only)

Placed **IN** the spec:

| Member | Signature | Sites | New dependency |
|---|---|---|---|
| entry point | `PlatformTestWebApplicationFactory<TEntryPoint> where TEntryPoint : class` | 63 + 117 | none |
| host seam | `ConfigureWebHost(Action<IWebHostBuilder>)` (inherited) | 57 | none |
| environment | `WithEnvironment(string environmentName)` | 46 | none |
| replace | `ReplaceService<TService>(TService instance)` | 24 | none |
| replace typed | `ReplaceService<TService, TImpl>() where TImpl : class, TService` | subset of 24 | none |
| decorate | `DecorateService<TService, TWrapper>(Func<TService, TService> decorate)` | 1 | none |
| clock | `WithClock(IClock clock)` | 4 | none — `IClock` is `Platform.Core` |
| teardown | `OnDispose(Action action)` | 13 | none |
| teardown async | `OnDispose(Func<ValueTask> action)` | 13 | none |
| temp root | `UseTemporaryDirectory()` / `TemporaryDirectory` property | 6 | none |
| permutations | supported by *not* requiring subclassing | 19 | none |

Deliberately **DEFERRED**, with the reason recorded:

| Deferred | Sites | Why deferred |
|---|---|---|
| `ReplaceDbContext<TContext>(Action<DbContextOptionsBuilder>)` | 19 | needs `Microsoft.EntityFrameworkCore`; changes the manifest and puts an ORM in the closure of a package 4 of 81 repos consume; prejudges the provider decision (**D2**) |
| `MigrateOnStartup()` / `EnsureCreatedAsync()` | 11 | same EF dependency, and the right schema strategy is provider-specific — `Migrate` vs `EnsureCreated` is exactly the choice the Postgres analysis says is unsettled |
| `ResetDatabaseAsync()` | 22 | the isolation lifecycle is inseparable from the provisioning model (container / local cluster / external server), which is out of scope |
| `CaptureLogs()` | 6 | `ConfigureWebHost(b => b.ConfigureLogging(…))` already covers it; a shared contract must fix scopes/providers/level opinions that differ per repo |
| `WithDefaultRequestHeader` | 3 | `CreateClient(WebApplicationFactoryClientOptions)` + `ConfigureClient` are inherited |
| `IAsyncLifetime` on the factory | 5 | would put xUnit in a shipped package, violating the inherited toolkit rule |
| fake auth scheme / startup-failure factory / auth-flow helpers | 2/2/6 | repository-specific policy; `ReplaceService` / `DecorateService` / `ConfigureTestServices` are the supported construction path |

**Why `WithClock(IClock)` and not `WithClock(TimeProvider)`:** `IClock` is
already in `Platform.Core`, which `Platform.Testing.AspNetCore` already
references, so this adds **zero** manifest movement and keeps the clock on the
platform's own abstraction. `TimeProvider` is a BCL type but not the platform's
abstraction; a `TimeProvider` overload is open decision **D3**.

## 5. Contracts and behavioural model

| Input / state | Required result |
|---|---|
| `TEntryPoint` is the app's `Program` or a public marker | the consumer's real `WebApplication` is booted over `TestServer` |
| `TEntryPoint` is `internal` and not visible to the test project | **named** failure naming the type and its assembly; never a silent fallback |
| `TEntryPoint` exists but exposes no host builder / entry point | **named** failure; never a silent fallback |
| `WithEnvironment("Staging")` | `IHostEnvironment.EnvironmentName == "Staging"` **and** the consumer's `appsettings.Staging.json` applies |
| no `WithEnvironment` call | `PlatformTestEnvironments.Testing` (unchanged default) |
| `ConfigureWebHost(b => b.UseSetting(k, v))` | the value is visible to the booted app |
| `ConfigureWebHost(b => b.ConfigureAppConfiguration(c => c.AddInMemoryCollection(…)))` | the overlay is visible and **wins** over the consumer's own file/host configuration |
| `ConfigureWebHost(b => b.ConfigureLogging(…))` | the consumer's logging pipeline is configured |
| `ReplaceService<T>(instance)` after the app registered `T` several times | every prior registration of `T` is removed; the supplied instance is the single resolution |
| `DecorateService<T, TWrapper>(f)` | the *original* `T` is passed to `f` and remains reachable inside `TWrapper` |
| `ReplaceService<T>` for a `T` that was never registered | **named** `InvalidOperationException`; never a silent no-op |
| `WithClock(clock)` | the app's `IClock` resolves to the supplied instance |
| any mutating call **after** the first host access | **named** `InvalidOperationException` pointing at "configure before first access, or use `WithWebHostBuilder`" |
| `OnDispose` actions, multiple, with one throwing | host stops first; actions run in **reverse** registration order; a throwing action does not prevent the rest, nor host disposal |
| `UseTemporaryDirectory()` | a unique, writable directory under the OS temp root; removed on disposal; never the process current directory; never a content root |
| two factories configured differently | two isolated hosts; no shared configuration, no shared service state |

`ConfigureWebHost` callbacks must run at **host** configuration time, after the
application's own `ConfigureServices`, which is what makes
`services.RemoveAll<T>()` inside `ReplaceService` actually remove the app's
registration. This is the single most important ordering property of the design
and the reason the class is a subclass rather than a wrapper.

**Content root changes meaning.** Today the class hard-codes
`ContentRootPath = AppContext.BaseDirectory` on a WebApplication it owns. With
a real application, the content root must resolve to the *consumer's* output
directory so that static files, Razor views and `appsettings.*.json` are found.
`Microsoft.AspNetCore.Mvc.Testing` already emits `MvcTestingAppManifest.json` for
this (present in this package's own `bin/`), so the mechanism exists and needs
no new code — but it is a real behaviour change for the 2 existing tests and MUST
be called out in migration, not discovered at implementation time.

## 6. Failure boundaries

| Case | Required outcome |
|---|---|
| marker type not visible from the test assembly (typical: `internal` `Program` with no `InternalsVisibleTo`) | named diagnostic naming the type and the declaring assembly, and naming `InternalsVisibleTo` or a public marker as the fix |
| marker type is not an entry point (no host builder reachable) | named diagnostic; **must not** boot an empty `WebApplication` |
| the application's own startup throws | the original startup exception surfaces to the test; the factory does not swallow or wrap it into a pass |
| a `ConfigureWebHost` callback throws | the exception propagates; no partially-configured host is left behind |
| `WithEnvironment(null)` / whitespace / empty | `ArgumentException` at the call site, before any host is built |
| `WithClock(null)` | `ArgumentNullException` at the call site |
| `OnDispose(null)` | `ArgumentNullException` at the call site |
| mutation after first host access | named `InvalidOperationException` (see §5) |
| `ReplaceService` for an unregistered service | named `InvalidOperationException` |
| a teardown action throws | recorded, remaining actions still run, host still disposed, exception surfaced to the caller of `DisposeAsync` |
| a consumer project references this package from production code | existing architecture guard `Production_projects_do_not_reference_any_testing_only_platform_project` fails — the guard is the enforcement, not a runtime check |

**The removed behaviour is the load-bearing one.** Today the class always
succeeds, because it always builds an empty app; a factory that cannot resolve
the entry point has no fallback. That is intentional: a silent empty-app
fallback is indistinguishable from a passing test suite, and it is the reason the
current class is unused.

## 7. Compatibility

| Consumer situation | Outcome |
|---|---|
| 63 local `WebApplicationFactory<T>` subclasses | **unchanged and still valid.** Adoption is opt-in per package; the platform is not a shared runtime service. They may migrate one at a time. |
| 117 anonymous `new WebApplicationFactory<T>().WithWebHostBuilder(…)` sites | **unchanged.** The shared type is a `WebApplicationFactory<T>`, so it is a drop-in for the same shape. |
| 19 config-permutation subclasses | **deletable in principle** — one generic factory configured per scenario replaces them. Actual deletion is consumer-side and out of scope; their exact keys are consumer-private. |
| production packages | **untouched**; four architecture guards stay green because no project reference and no package reference is added. |
| `eng/package-manifest.json` | **byte-identical.** The manifest records package identity, `projectReferences`, `packageReferences`, `frameworkReferences`, TFMs, version and description — not public members. Verified: `Platform.Testing.AspNetCore` is already present at `eng/package-manifest.json:1366-1386`. |
| 75-package inventory | unchanged at 75; `scripts/package-inventory.sh` (which enumerates `src/*/*.csproj`) output is unchanged. |
| `eng/public-api-baseline.txt` | **regenerated.** It previously scanned only `Platform.Core`, `Platform.Billing.Contracts`, `Platform.Identity.Contracts`, `Platform.Admin.Contracts`, `Platform.Eventing.Contracts`, so this package's public surface was unguarded. The script now scans this package too and the baseline was regenerated (**D4**). |
| in-repo consumers of the non-generic class | 6 real sites (README example, two docs, two test files). Migration cost is small but **not zero** — see **D1**. |

**Inherited-and-unchanged (do not re-specify, do not regress):** single-key
in-memory config (`WithConfiguration`), the services+config callback
(`ConfigureTestServices`), `CreateClient()` over `TestServer`, `IAsyncDisposable`
teardown, the `Testing` environment constant, `PlatformTestHostConfiguration`.

## 8. Migration

1. Rewrite the class in place; keep the `Testing` constant and both existing
   extension points.
2. Rewrite `PlatformTestWebApplicationFactoryTests.cs`: the 2 tests that assert
   the empty-app `"/"` → `"ok"` round-trip have no meaning under a real-app
   factory and become a fixture-app test — the factory is pointed at an
   in-repo test application (the starter-template fixture at
   `templates/platform-application-starter/tests/StarterApp.Tests/` already
   demonstrates a real host) instead of at nothing.
3. Update `CheckoutScenarioTests.cs` (1 call site) and the README/docs examples.
4. Delete the 19 config-permutation classes **in the consumer repositories** —
   documented, not performed here.
5. Regenerate nothing: `eng/package-manifest.json` does not change.

**Rollback:** the change is one file plus docs and tests. Reverting restores the
empty-`WebApplication` factory, which no consumer depended on, so rollback is
costless to consumers and local to this repository.

## 9. Verification oracle

- `openspec validate shared-test-web-factory --strict` passes.
- `./scripts/generate-package-manifest.sh --check` reports
  `Manifest matches source.` — proving no package reference was added.
- `./scripts/check-public-api.sh` produces a clean diff.
- `scripts/package-inventory.sh` still enumerates 75 projects.
- `dotnet test tests/Platform.Testing.Tests` — the rewritten factory tests, with
  the empty-app assertions removed and a real-host assertion added.
- `dotnet test tests/Platform.Architecture.Tests` — all 350 guards green,
  including the four `Platform.Testing.AspNetCore` guards.
- `./scripts/quality-gate.sh` (restore, build, test, both `openspec validate
  --strict` passes, `git diff --check`).
- A named-diagnostic test for the inaccessible-marker case, asserting the
  message names the type and the assembly. This is the single most important new
  test, because it is the failure mode the old class could not have.

## 10. Decision ledger

### Resolved in this change

- **(a) Subclass vs wrapper** → **subclass `WebApplicationFactory<TEntryPoint>`**;
  inherited surface is free and all 117 anonymous sites keep working.
- **(b) Generic parameter location** → on the factory itself; C# has no default
  type parameter, so this is the only way to serve the 7 non-`Program` markers.
- **(c) New package vs rewrite in place** → **rewrite in place**; the package
  already has the right dependencies, and a new package would take 75 → 76 for
  zero delivered capability.
- **(d) Database fixture** → **out of scope**; 15 implementations, three
  incompatible provisioning models, and a Testcontainers `4.15.0` vs `3.10.0`
  version conflict under two package-id spellings. The seam is designed; the
  fixture is not.
- **(e) `IAsyncLifetime`** → **deferred**; xUnit in a shipped package violates
  the inherited framework-independence rule.
- **(f) Clock abstraction** → **`Platform.Core.IClock`**, because it is already
  referenced and adds zero manifest movement.
- **(g) D1 - non-generic compatibility shim** -> **deleted outright.** Measured
  across `/home/paul/code/*/` (excluding `obj/`, `bin/`, `node_modules/`,
  `target/`, `.git/`): **zero** external code consumers of the non-generic
  `PlatformTestWebApplicationFactory`. The only external mentions are two prose
  lines - `therapist-commons/docs/superpowers/plans/practice-foundation.md:41`
  and `ARCHITECTURE.md:334` - and one *package* reference,
  `therapist-commons/tests/Api.Tests/Api.Tests.csproj:21`, whose test project
  contains **no** reference to the type or its namespace (it uses its own
  `new WebApplicationFactory<Program>()` at `tests/Api.Tests/ApiTestFixture.cs:69`).
  An `[Obsolete]` shim would therefore have had zero external callers, so the six
  in-repo call sites were updated instead.
- **(h) D2 - where EF-backed helpers live** -> **still open**, deliberately. The
  guard gap it exposed *is* closed in this change: `Platform.Testing.AspNetCore`
  was absent from the architecture suite's `ProductionProjects` list, so the
  EF-forbidden rule did not cover it. It is now listed.
- **(i) D3 - `TimeProvider` overload** -> **added.** One repository uses
  `TimeProvider` directly, the overload adds no package reference, and
  `TimeProvider` is a BCL type. The cost is one method.
- **(j) D4 - public API baseline coverage** -> **closed for this package.**
  `eng/public-api-baseline.txt` covered only five projects and did not cover
  `Platform.Testing.AspNetCore`, so this change's public surface would otherwise
  have been unguarded. `scripts/check-public-api.sh` now scans the package too
  and the baseline was regenerated.
- **(k) D5 - decorate/replace ordering** -> **last registration wins, per service
  type.** Each `ReplaceService` / `DecorateService` call removes every prior
  registration of the target type and appends its own, so the last call made
  before the first host access is the effective one. The combination is not
  detected or forbidden: a consumer who both decorates and replaces the same
  contract gets the replacement.
- **(l) D6 - `WithEnvironment` argument** -> **arbitrary non-empty string.** 46
  repositories pin their own environment name; an allow-list would reject them.
- **(m) D7 - temp-directory scope** -> **per-factory by default, with an explicit
  process-wide opt-in** (`PlatformTestTemporaryDirectoryScope`). xUnit runs test
  collections in parallel, so a shared directory must be chosen deliberately; the
  shared directory is reference-counted and removed when the last participating
  factory is disposed.

### Adopted from an external MIT reference after measurement

- **(n) One host per factory instance** -> **adopted as a discipline**, and this
  factory already behaves that way. Measured across the workspace this session:
  **79** `protected override void ConfigureWebHost` declarations (69 files) and
  **118** `WithWebHostBuilder(` call sites (106 files across 17 repositories).
  (The amendment quoted 78 and 117; the measured values are what is recorded
  here.) Each `WithWebHostBuilder` call derives a *new* factory, so each of those
  sites also pays for a second host build: the duplication is a runtime cost, not
  only a maintenance one.
  The pattern was read in `fullstackhero/dotnet-starter-kit`
  (`/home/paul/learning/dotnet-starter-kit`, **MIT**, (c) 2021-2026), whose
  `src/Tests/Integration.Middleware.Tests/Infrastructure/MiddlewareWebApplicationFactory.cs`
  carries the note *"This factory NEVER calls WithWebHostBuilder - exactly ONE host
  is built per process"*, and which forces early host creation with `_ = Server;`
  so no `HttpClient` leaks. Its sibling factories take constructor parameters
  rather than hard-coded literals, which is the same parameterize-one-factory idea
  as the permutation requirement.
  **No code was imported or vendored.** That repository is a product starter kit
  carrying Finbuckle multitenancy, Hangfire, S3/MinIO and billing dependencies,
  which this library must not acquire. It is cited as a design source only; every
  line in this package is written for this package.
  No new API was added for this: the fluent
  `ConfigureWebHost(Action<IWebHostBuilder>)` already lets a consumer configure
  this factory at construction time without deriving a factory, and the
  post-first-access guard already rejects configuration that would require a second
  host. The requirement is satisfied by the existing surface plus the guard.
- **(o) Postgres container-pair and migration-lock patterns from that same
  reference** -> **deferred with the rest of the Postgres work.** That reference
  starts a PostgreSQL *and* a MinIO container per factory and serialises
  migrations behind a `SemaphoreSlim`. Both belong to the deferred fixture change:
  they presuppose a provisioning model, and they carry the same Testcontainers
  `4.15.0` vs `3.10.0` conflict under two package-id spellings that already blocks
  that change. Nothing about them is specified here.

### Open — owner decision required (D1, D3, D4, D5, D6, D7 resolved above; **D2 remains open**)

- **D1. Non-generic compatibility shim.** The class becomes
  `PlatformTestWebApplicationFactory<TEntryPoint>`. C# permits `Foo` and
  `Foo<T>` to coexist, so a non-generic obsolete shim is *possible*, but it is
  confusing at the call site. Three real internal call sites plus 4 doc sites
  must be migrated either way, and no external consumer uses the class today.
  Options: **(i)** delete the non-generic shape outright and update the 6
  internal sites — recommended; **(ii)** keep an `[Obsolete]` non-generic shim
  for one release; **(iii)** rename the generic type to
  `PlatformWebApplicationFactory<TEntryPoint>` and keep the old name untouched.
  **Resolved: option (i).** Measured consumer count is zero, so the non-generic
  shape is deleted rather than deprecated. See ledger entry **(g)**.
- **D2. Where EF-backed helpers eventually live.** If `ReplaceDbContext` /
  `MigrateOnStartup` / `ResetDatabaseAsync` are later approved, they need
  `Microsoft.EntityFrameworkCore`. Options: **(i)** a new
  `Platform.Testing.AspNetCore.EfCore` package (75 → 76; cleanest boundary, and
  consistent with `Platform.Persistence.EfCore`); **(ii)** add the reference to
  `Platform.Testing.AspNetCore` (manifest changes, ORM enters the closure of a
  widely-consumed test package); **(iii)** leave them to consumers via
  `ConfigureWebHost` indefinitely. *Also entangled with the unresolved
  Testcontainers version conflict.*
  Note: neither (i) nor (ii) tripped an existing architecture guard —
  `Platform.Testing.AspNetCore` was **not** in the architecture suite's
  `ProductionProjects` list, so the EF-forbidden rule did not cover it. **That gap
  is closed in this change**: the package is now listed, so a future EF dependency
  here fails the guard. See ledger entry **(h)**.
- **D3. `TimeProvider` overload.** `WithClock` is bound to `Platform.Core.IClock`.
  **Resolved: added.** `WithClock(TimeProvider)` ships alongside
  `WithClock(IClock)`. See ledger entry **(i)**.
- **D4. Public API baseline coverage.** `eng/public-api-baseline.txt` does not
  cover `Platform.Testing.AspNetCore`, so this change's public surface is
  unguarded by `check-public-api.sh`. Should the testing packages be added to
  the baseline project list, or is a testing package deliberately outside the
  published-surface policy?
  **Resolved for this package:** `scripts/check-public-api.sh` now scans
  `Platform.Testing.AspNetCore` and the baseline was regenerated. Whether the
  *remaining* test-only packages belong in the published-surface policy is a
  separate, wider question and is **not** decided here. See ledger entry **(j)**.
- **D5. `ReplaceService` last-registration and decoration ordering.**
  **Resolved: last registration wins, per service type.** See ledger entry
  **(k)**.
- **D6. Whether `WithEnvironment` accepts an arbitrary string or a validated
  allow-list.** **Resolved: any non-empty string is accepted.** See ledger entry
  **(l)**.
- **D7. Temp-directory scope.** **Resolved: per-factory by default, with an
  explicit process-wide opt-in.** See ledger entry **(m)**.

### Assumptions

- The repository remains a private, independently adoptable library set, not a
  shared runtime service (`openspec/config.yaml`); adoption stays opt-in, so no
  consumer is forced off its local factory.
- `IClock` in `Platform.Core` is the platform's intended clock seam and will not
  be replaced by `TimeProvider` as a default.
- The measured shape counts are stable enough to size the API; they were
  spot-checked this session (24 repos, 63 of 64 subclass definitions, 7 non-`Program`
  markers all reproduce) and are carried from the prior analysis otherwise.
- `Microsoft.AspNetCore.Mvc.Testing` remains the only ASP.NET testing package
  reference; nothing in this change needs a newer version.

### Blockers

None for the scope as written. The blockers are **D1** and **D2**, which are
owner decisions on public API naming/deprecation and on where an ORM reference
may live. Both are avoidable by implementing the IN-scope surface first; neither
blocks the rest of the change.
