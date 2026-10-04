# Proposal: Make the shared ASP.NET test factory usable, and delete the config-permutation duplication

## Why

`Platform.Testing.AspNetCore` ships a web test factory that **none of the repos
that need one can use**.

`src/Platform.Testing.AspNetCore/PlatformTestWebApplicationFactory.cs:74-89`
builds its *own* empty `WebApplication` — `WebApplication.CreateBuilder` plus a
single `app.MapGet("/")` — and hands back a `TestServer` over it. It is not a
`WebApplicationFactory`, and it is not a narrow one. Across the workspace there
are **63 local `WebApplicationFactory<T>` subclass definitions in 24 repos**
(plus 1 template inside this repository), and **every one of them boots the
consumer's real application** through `ConfigureWebHost` / `CreateHostBuilder`.
Not one of them does what the shared class does. The whole surface below the
`ConfigureWebHost` line — real bootstrap, real DI graph, real middleware
pipeline, real `Program` resolution — is absent, so adopting the shared class
today would mean testing a `MapGet("/")` and calling it the application's web
host.

The cost is measured, not estimated (spot-checked this session: 24 repos and 63
of the 64 subclass definitions reproduce exactly; the entry-point distribution
reproduces as 409 `WebApplicationFactory<Program>` against **7 non-`Program`
markers** — `Arivio.Api.Program`, `Stylify.Web.ProgramMarker`,
`MinimalWebSampleMarker`, `IdentitySampleMarker`,
`SomadaNote.Web.WebAssemblyMarker`, `SomadaNote.Api.ApiAssemblyMarker`,
`HistoryKB.Web.Program`):

| Shape present in consumer factories | n of 63 | Shared class today |
|---|---|---|
| Config overlay (`UseSetting` / `AddInMemoryCollection`) | 57 | partial — single key only, no `ConfigureWebHost` |
| Environment pinning (`UseEnvironment`) | 46 | **no** — hard-coded `PlatformTestEnvironments.Testing` |
| Config-permutation matrix (one class per scenario) | ~29 | **no** — a permutation *forces* a subclass today |
| Per-factory unique DB identity | 28 | **no** |
| Remove-and-replace a service with a fake (`RemoveAll<T>` + `Add*`) | 24 | partial — the `Add*` half only, via `ConfigureTestServices` |
| Real Postgres/SQLite + isolation lifecycle | 22 | **no** |
| EF provider re-registration (`DbContextOptions` swap) | 19 | **no** |
| Dispose-time cleanup | 13 | **no** |
| Unreachable connection string (host must start DB-free) | 12 | **no** |
| Async schema bootstrap | 11 | **no** |
| Workers / hosted services disabled | 8 | **no** (reachable through `ConfigureWebHost` once it exists) |
| Named entry-point marker | 7 | **no** — the class is non-generic |
| Temp storage root · logging capture · real-auth-flow helpers | 6 each | **no** |
| `IAsyncLifetime` on the factory | 5 | **no** — and out of scope by design (see Non-goals) |
| Clock swap · `ConfigureClient` headers | 4 / 3 | **no** / partial |
| Fake auth scheme/principal · startup-failure factories | 2 each | **no** |

Two more consumers exist as **117 anonymous `new WebApplicationFactory<T>()` +
`WithWebHostBuilder(...)` sites in 106 files** using the same shapes. They
inherit everything they need today and would keep working only if the shared
factory *is* a `WebApplicationFactory<T>`; a wrapper would force all 117 to be
rewritten for no gain.

**19 of the 63 subclasses exist purely as config permutations** (hestiaLab 10,
linglitch 6, argoscope 3). They differ only by a configuration key or an
in-memory database name. That is 19 classes, 19 files, and a permanently
diverging copy of bootstrap logic that exist to express a dictionary.

## What Changes

- **Make the factory a real `WebApplicationFactory<TEntryPoint>` subclass** so it
  boots the consumer's application instead of an empty one. The generic
  parameter carries the entry point, which is what covers the 7 non-`Program`
  markers and the 2 namespaced `Program` types.
- **Expose the `ConfigureWebHost` seam** (`UseSetting`,
  `ConfigureAppConfiguration`, `ConfigureLogging`, `ConfigureServices`,
  content root, `CreateHost`) so the 57-site config-overlay shape and the
  8-site hosted-services shape need no new API. It is inherited, not
  reinvented.
- **Add `WithEnvironment(string)`** so the 46 factories that pin a non-default
  environment stop doing it by hand; `PlatformTestEnvironments.Testing` stays
  the default.
- **Add `ReplaceService<TService>(TService)`, `ReplaceService<TService, TImpl>()`
  and `DecorateService<TService, TWrapper>(Func<TService, TService>)`** to close
  the remove-and-replace shape (24 sites + 1 decoration) that
  `ConfigureTestServices` only half covers.
- **Add `WithClock(IClock)`** bound to the already-referenced `Platform.Core`
  abstraction — no new dependency, 4 sites.
- **Add `OnDispose(Action)` / `OnDispose(Func<ValueTask>)` and
  `UseTemporaryDirectory()`** as the provider-agnostic resource-lifecycle seam
  (13 + 6 sites) that a future database fixture can register its own cleanup
  through.
- **Delete the need for config-permutation subclasses**: one generic factory,
  configured per scenario, replaces the 19 hestiaLab/linglitch/argoscope
  classes.
- **Update `docs/packages.md` § Platform.Testing.AspNetCore,
  `docs/platform-testing-toolkit.md` and the `README.md` row/example** to
  describe the new surface.

## Public API Impact

**Yes — this is a public API change, in a test-only package.**

| Item | Change |
|---|---|
| `Platform.Testing.AspNetCore` package identity / version | **unchanged** (same `VersionPrefix`) |
| `PlatformTestWebApplicationFactory` | becomes generic `PlatformTestWebApplicationFactory<TEntryPoint>`; base type changes from `IAsyncDisposable` to `WebApplicationFactory<TEntryPoint>` |
| New public members | `WithEnvironment`, `ReplaceService` ×2, `DecorateService`, `WithClock`, `OnDispose` ×2, `UseTemporaryDirectory`, `TemporaryDirectory` |
| Inherited-and-therefore-available | `ConfigureWebHost`, `WithWebHostBuilder`, `CreateClient()`, `CreateClient(WebApplicationFactoryClientOptions)`, `ConfigureClient`, `Services`, `Server` |
| Production package surface | **unchanged** — no `src/` production package is touched |
| Package count | **75 → 75**, unchanged |
| `eng/package-manifest.json` | **unchanged** — the manifest records package identity, references and target frameworks, not public members, and this change adds no package reference |
| `eng/public-api-baseline.txt` | **unchanged** — the baseline covers only `Platform.Core`, `Platform.Billing.Contracts`, `Platform.Identity.Contracts`, `Platform.Admin.Contracts` and `Platform.Eventing.Contracts`; `Platform.Testing.AspNetCore` is not in it |

## BFS Impact Map

- **Capabilities:** new `platform-test-web-factory`. `platform-testing-toolkit`
  is **not modified** — its shipped requirements concern determinism, framework
  independence and infrastructure-prerequisite honesty, and none of their
  wording changes (see `design.md` §3).
- **Code:** `src/Platform.Testing.AspNetCore/` only (the factory type and, if the
  compatibility path is taken, a non-generic shim). `docs/packages.md`,
  `docs/platform-testing-toolkit.md`, `README.md`.
- **Tests:** `tests/Platform.Testing.Tests/AspNetCore/` (5 existing tests, 2 of
  which assert the empty-`WebApplication` behaviour and **must be rewritten**,
  not kept) and `tests/Platform.Testing.Tests/Scenarios/CheckoutScenarioTests.cs`
  (1 real call site).
- **Internal blast radius is not zero:** 6 code/doc sites in this repository use
  the current non-generic class (`README.md:583-601`,
  `docs/packages.md:736`, `docs/platform-testing-toolkit.md:10,70`,
  `tests/.../PlatformTestWebApplicationFactoryTests.cs`,
  `tests/.../CheckoutScenarioTests.cs`), plus narrative mentions in `AGENTS.md`
  and `HANDOFF.md`. The compatibility decision is in `design.md` §6 and is an
  **open decision**, not a settled one.
- **Architecture guards (350 tests in `Platform.Architecture.Tests`):** four are
  directly load-bearing and MUST stay green —
  `Platform_Testing_AspNetCore_only_depends_on_public_contracts_and_core`,
  `Platform_Testing_AspNetCore_declares_FrameworkReference_for_ASPNET`,
  `Production_projects_do_not_reference_Platform_Testing_toolkit`,
  `Production_projects_do_not_reference_any_testing_only_platform_project`.
  All four are satisfied by *not adding a project reference and not adding a
  package reference* — which is why the EF Core helpers are deferred.
- **Manifest / inventory reflection:** because the package is already in
  `eng/package-manifest.json` (with `packageReferences: [Microsoft.AspNetCore.Mvc.Testing]`
  and `projectReferences: [../Platform.Core]`) and the change adds no reference,
  `./scripts/generate-package-manifest.sh --check` stays green with a
  byte-identical file. The 75-package inventory in `docs/packages.md` **does**
  change, because it describes members, not references — the per-package
  surface list and the `tests/Platform.Testing.Tests` row must be rewritten.
- **Failure surface:** an inaccessible or non-entry-point marker type, a
  configuration callback that throws, a `ReplaceService` for a service that was
  never registered, a mutation after the host has been created, and a throwing
  teardown action — each has a named, specified outcome in the spec. The
  behaviour that is **removed** is the important one: the class no longer ever
  boots a fallback empty application.
- **Compatibility:** consumers that subclass `WebApplicationFactory<T>` directly
  are unaffected — they can keep their own factories, since the platform is not
  a shared runtime service and adoption stays opt-in per package. The 117
  anonymous sites are unaffected because the shared type *is* a
  `WebApplicationFactory<T>`.
- **Out of scope but adjacent:** the Postgres fixture conflict (see Non-goals).

## Capabilities

### New Capabilities

- `platform-test-web-factory`: a shared, test-only ASP.NET Core factory that
  boots a consumer's real application from a named entry point and exposes
  environment selection, service replacement/decoration, deterministic clock
  injection and provider-agnostic resource-lifecycle hooks — with no test
  framework, ORM or container dependency.

### Modified Capabilities

- None. `platform-testing-toolkit` keeps its shipped wording; this change adds a
  sibling capability rather than rewriting an archived contract.

## Non-goals

- **No Postgres/SQLite fixture.** A separate analysis found 15 distinct
  implementations across the workspace with **three incompatible provisioning
  models** (Testcontainers container / local `initdb` + `pg_ctl` cluster /
  externally-managed server) and a hard version conflict: `chronicleaf` pins
  Testcontainers `4.15.0` while three other repos pin `3.10.0`, under two
  different package-id spellings. Choosing a provisioning model is a separate
  change with its own compatibility surface. This change is designed so one can
  plug in later — a fixture is handed a connection string through the ordinary
  config overlay, a schema strategy, and an `OnDispose` registration — but the
  fixture itself, and the Testcontainers version resolution, are **not**
  specified here.
- **No `ReplaceDbContext<TContext>`, no `MigrateOnStartup()` /
  `EnsureCreatedAsync()`, no `ResetDatabaseAsync()`** (19 / 11 / 22 sites).
  These need `Microsoft.EntityFrameworkCore` in `Platform.Testing.AspNetCore`,
  which would change `eng/package-manifest.json`, pull an ORM into the
  dependency closure of a package 4 of 81 repos consume, and prejudge the
  deferred provider decision. Consumers reach the same behaviour today through
  `ConfigureWebHost(b => b.ConfigureServices(s => …))`.
- **No `IAsyncLifetime` on the factory** (5 sites). The shipped
  `platform-testing-toolkit` requirements state that test helpers SHALL support
  assertions *without a framework dependency* and that testing packages SHALL
  not depend on assertion packages; an `IAsyncLifetime` implementation would
  put xUnit in a shipped package. A consumer's `IClassFixture` or collection
  fixture disposes the factory instead.
- **No `CaptureLogs()`** (6 sites). `ConfigureWebHost(b =>
  b.ConfigureLogging(…))` already covers the shape, and a shared capture
  contract would have to fix opinions — scopes, provider set, level — that
  differ per repository.
- **No `WithDefaultRequestHeader`** (3 sites). `CreateClient(WebApplicationFactoryClientOptions)`
  and `ConfigureClient` are inherited from `WebApplicationFactory<T>` and cover it.
- **No fake auth scheme/principal, startup-failure factory, or real-auth-flow
  helper** (2 / 2 / 6 sites). Each is repository-specific policy; the generic
  `ReplaceService` / `DecorateService` / `ConfigureTestServices` seams are the
  supported way to build them.
- **No migration of any consumer repository.** 24 repos are described, zero are
  edited. The platform is not a shared runtime service
  (`openspec/config.yaml`); migration is documented, not performed.
- **No change to the 350 architecture-guard tests**, to
  `eng/public-api-baseline.txt`, to `scripts/generate-package-manifest.sh`, or
  to the `net10.0` baseline.
- **No implementation in this change** — planning artifacts only.

## Package Boundary and Split Assessment

**Single outcome:** a consumer writes one generic factory instead of a
hand-rolled `WebApplicationFactory<T>` subclass, and the 19 config-permutation
subclasses disappear.

**Included:** the generic entry-point parameter, the real-app bootstrap, the
`ConfigureWebHost` seam, `WithEnvironment`, `ReplaceService` ×2,
`DecorateService`, `WithClock(IClock)`, `OnDispose` ×2,
`UseTemporaryDirectory`, the rewritten tests, and the three documentation
surfaces.

**Excluded:** any database fixture, any ORM reference, any test-framework
reference, any consumer-repository edit, and any new package.

**Split signals considered — not split.** These are one owner (this repository),
one lifecycle (construct → boot → exercise → dispose), one type, and one oracle
(a consumer writes one factory and its existing tests pass unchanged). Splitting
`ReplaceService` out of the factory would leave a factory that cannot host a
test. Splitting the environment parameter out of the generic type is impossible
in C#: **a class cannot have a default type parameter**, so
`PlatformTestWebApplicationFactory<TEntryPoint = Program>` is not available and
the generic parameter is load-bearing for the 7 non-`Program` markers.

**Boundary decision recorded:** no new package. `Platform.Testing.AspNetCore`
is already the correct owner — it already carries
`Microsoft.AspNetCore.Mvc.Testing`, already declares the ASP.NET framework
reference, and already depends only on `Platform.Core`. A new
`Platform.Testing.AspNetCore.EfCore` would be the *right* answer if EF helpers
are later added, but adding it now would take the inventory from 75 to 76 for
zero delivered capability.

**Dependencies:** consumes the shipped `dotnet10-platform-baseline` (`net10.0`),
`platform-testing-toolkit` (determinism + framework-independence rules), and
`Platform.Core` (`IClock`). Blocks nothing. Blocked by nothing.

## Sibling and Shared Architecture Reconnaissance

**Question:** is the web test factory a shared platform capability, or does it
stay a per-repository concern?

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository | `src/Platform.Testing.AspNetCore/PlatformTestWebApplicationFactory.cs:74-89` | `TestServer` plumbing, `IAsyncDisposable` teardown, `Testing` environment constant, `ConfigureTestServices` | boots an empty `WebApplication`; not a `WebApplicationFactory`; no generic entry point | dotnet-platform-libs | **rewrite in place — extend the shared owner** |
| this repository | `openspec/specs/platform-testing-toolkit/spec.md` | "Test dependencies SHALL not leak into production packages"; "Integration prerequisites SHALL be explicit" | states the rules but no web-factory requirement | dotnet-platform-libs | **consume; add a sibling capability, do not modify** |
| this repository | `templates/platform-application-starter/tests/StarterApp.Tests/SiteUserAuthTests.cs:144` | a working `class Factory : WebApplicationFactory<Program>` in-repo | is the 64th local copy, not the shared type | dotnet-platform-libs | **adopt as the in-repo consumer fixture** |
| this repository | `src/Platform.Web/PlatformWebAssemblyMarker.cs` | the established public-marker pattern for integration hosts | marker lives in `Platform.Web`; consumers with no `Platform.Web` need their own | dotnet-platform-libs | **adopt the pattern; do not move the type** |
| this repository | `eng/package-manifest.json:1366-1386`, `eng/public-api-baseline.txt` | generated inventory + 5-project API baseline | baseline does not cover this package, so API drift is unguarded here | dotnet-platform-libs | **manifest stays byte-identical; extend `docs/packages.md` instead** |
| 24 consumer repos | 63 `WebApplicationFactory<T>` subclasses, 19 of them config permutations | real-app bootstrap, env pinning, service replacement, DB lifecycle | the shared type cannot express any of it | each repo | **describe and document; edit none** |
| 106 consumer files | 117 anonymous `new WebApplicationFactory<T>().WithWebHostBuilder(…)` sites | rely on inherited `ConfigureWebHost` / `CreateClient` | none — a `WebApplicationFactory<T>` subclass satisfies them as-is | each repo | **no change required** |
| workspace DB-fixture owners | 15 Postgres implementations, Testcontainers `4.15.0` vs `3.10.0`, two package-id spellings | connection strings, schema bootstrap, isolation lifecycle | three incompatible provisioning models + a version conflict | each repo / workspace-governance | **out of scope; design the plug-in seam only** |

**Decision (recorded):** the web test factory stays a **shared
`Platform.Testing.AspNetCore` capability**, rewritten in place. It is not a new
package and not a workspace-governance artifact: the duplication it removes lives
in 24 repositories that all already take this package's dependency direction for
granted, and the platform's own rule (`openspec/config.yaml`: each library
compiled into the consuming application, adoptable independently) is satisfied by
one test-only package that no production project references.

## Evidence Boundary

This change can prove, inside this repository, that the factory boots a real
application from a named entry point, that the entry-point and environment
contract behaves as specified including the failure cases, that service
replacement and decoration are observable, that clock and teardown hooks fire in
the specified order, and that the package stays framework-independent with the
manifest and the 75-package inventory consistent.

It **cannot** prove that any of the 24 consumer repositories compiles, passes, or
migrates; that no consumer needs an API left off the deferred list; or that the
19 config-permutation classes are actually deletable in their own repositories,
since their exact configuration keys are consumer-private. Consumer-side proof
stays with each consumer's own build. Nothing in this change reports a
container-backed or database-backed integration as verified — that evidence
belongs to the deferred fixture change.
