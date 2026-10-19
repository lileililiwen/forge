# Design: One consumer bootstrap for the platform

## 1. Implementation boundary

**Repository:** `/home/paul/code/dotnet-platform-libs` — C#/.NET (SDK `10.0.400`,
`net10.0`).

**Files to change:** `build/Platform.Consumer.props` (and a
`Platform.Consumer.targets` only if a target-time hook is required), the
repository `Directory.Build.props` / `Directory.Packages.props`, `global.json`,
the release workflow, the conformance fixture under
`tests/Platform.ConsumerConformance/`, `docs/workspace-consumer-bootstrap.md`,
and `docs/platform-product-adoption.md`.

**Must not change:** `src/Platform.*` public APIs, package identities, the
workspace `Directory.Build.props`/`nuget.config` (documented contract only), or
any consumer repository.

## 2. Language and runtime

C# on .NET SDK `10.0.400`, `TargetFramework` `net10.0`. MSBuild conditions must
use the same target.

- `dotnet restore Platform.sln`
- `dotnet build Platform.sln -c Release`
- `dotnet test Platform.sln -c Release`
- `dotnet pack Platform.sln -c Release -o artifacts/packages`
- `./scripts/conformance.sh`
- `./scripts/quality-gate.sh` and `openspec validate --changes --strict --no-interactive`

## 3. Ownership and shared code

Repository-local. The workspace switch and feeds are **consumed** as a
documented contract; this repository owns how a consumer resolves the platform.

- **Dependency direction:** `Platform.*` packages never depend on consumer
  applications; the bootstrap injects a reference into the consumer, never the
  reverse. `Platform.Core` stays free of ASP.NET Core and EF Core.
- **Package ownership:** unchanged; no package is added.
- **Smallest adoptable boundary:** the bootstrap must remain a single imported
  `.props`/`.targets` pair that a consumer can opt out of with one property.
- **Release boundary:** ships in the platform's own pack/publish flow.

## 4. Behavioral model

| Consumer state | Result |
|---|---|
| `PlatformAsSource=true`, local checkout present | `ProjectReference` to the platform source |
| `PlatformAsSource=true`, checkout missing | named error at evaluation; no silent package fallback |
| `PlatformAsSource=false`, feed reachable | `PackageReference` at the pinned version |
| `TargetFramework` not supported | named diagnostic naming the consumer's target and the supported target |
| `PlatformConsumerOptOut=true` | no injection, no defaults, no diagnostic |

Consumer defaults applied when not opted out: `Nullable=enable`,
`LangVersion` latest supported, analyzer level, `TreatWarningsAsErrors`, and
central package management participation. Each default is overridable by the
consumer.

## 5. Contract and compatibility

- Public package APIs: unchanged (see the proposal's Public API impact).
- The bootstrap contract is documented in
  `docs/workspace-consumer-bootstrap.md`: the switch property, the opt-out
  property, the supported target, the feed, and the version source.
- **Compatibility:** consumers that already set `PlatformConsumerOptOut` or pin
  versions explicitly keep their behaviour; the change makes the documented
  switch real rather than changing the documented surface.

## 6. Failure and boundary policy

| Case | Result |
|---|---|
| `PlatformAsSource=true` without a checkout | evaluation error naming the path and the switch |
| Unsupported target framework | named diagnostic; no injection |
| Feed unreachable in package mode | restore error; no fallback to source |
| Publish without credentials | release job fails loudly; never a silent skip |
| Opt-out set | no injection and no defaults, no diagnostic |
| Version unset | error naming `PlatformPackageVersion` |

## 7. Verification oracle

- `./scripts/conformance.sh` packs the platform and restores, builds and tests a
  fixture consumer from the feed in package mode, and from source in
  `PlatformAsSource=true` mode.
- A fixture consumer on `net8.0` produces the named unsupported-target
  diagnostic and does not silently succeed.
- A fixture consumer with `PlatformConsumerOptOut=true` receives neither
  reference nor defaults.
- `dotnet build Platform.sln -c Release` and `dotnet test Platform.sln -c
  Release` pass; `./scripts/quality-gate.sh` and strict OpenSpec validation pass.
- The release workflow pushes a versioned package to the declared feed.

## 8. Decision ledger

**Assumptions:** the platform stays `net10.0`-only; the workspace feeds and
`GITHUB_TOKEN` credential expansion are the intended publish target; consumers
migrate on their own schedule.

**Resolved alternatives:**

- (a) Honour `PlatformAsSource` in the hook vs remove the switch → chose
  **honour it**, because the workspace and docs already promise it.
- (b) Inject only `Platform.Core` vs let the consumer add packages → keep the
  current minimal injection and rely on `PackageReference`/explicit
  `ProjectReference` for the rest; adding more implicit references would widen
  the smallest adoptable boundary.
- (c) Propagate defaults implicitly vs require opt-in → chose **defaults with an
  explicit opt-out**, matching the existing opt-out property.
- (d) Publish inside `release.yml` vs a separate workflow → **extend
  `release.yml`**, which already packs.
- (e) Silently skip unsupported targets vs diagnostic → chose **diagnostic**, so
  a default-target consumer cannot think it adopted the platform.

**Deferred:** the memory that `mewo` targets `net8.0` is a consumer-side
migration, named as follow-up only.

**Blockers:** none.
