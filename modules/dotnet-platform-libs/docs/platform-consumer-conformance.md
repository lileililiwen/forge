# Platform consumer conformance

The consumer conformance fixture is a separate test project that consumes the
platform packages from a locally produced package feed rather than a
cross-repository `<ProjectReference>`. The fixture verifies that the packages
can be adopted together, that registered services can be replaced by the
consumer, and that the documented safety and isolation guarantees hold against
the published artifacts.

## Layout

```
scripts/conformance.sh                            # pack, restore, build, test
tests/Platform.ConsumerConformance/
  Platform.ConsumerConformance.csproj             # PackageReference only
  Directory.Build.props                           # disables central version mgmt
  nuget.config                                    # local-platform-feed + nuget.org
  .gitignore                                      # excludes .local-feed/ and .logs/
  .local-feed/                                    # generated, ignored
  .logs/                                          # generated, ignored
  PlatformConsumerConformanceAssemblyMarker.cs
  PlatformConsumerConformanceAssemblyMarkerTests.cs
  Fixtures/
    ConsumerServiceCollectionFactory.cs
    ConsumerTestHostFactory.cs
    contracts/                              # canonical shared-contract envelopes
      ContractFixtureReader.cs              # deterministic local-JSON validator
      identity-subject.valid.json / .invalid.json
      permission.valid.json / .invalid.json
      tenant.valid.json / .invalid.json
      audit.valid.json / .invalid.json
      gate-result.valid.json / .invalid.json
      release-evidence.valid.json / .invalid.json
  Tests/
    ContractEnvelopeConformanceTests.cs
    PackageFeedVerificationTests.cs
    AspNetCoreRegistrationTests.cs
    RateLimitingRegistrationTests.cs
    IdempotencyRegistrationTests.cs
    JobsRegistrationTests.cs
    MailingRegistrationTests.cs
    EventingRegistrationTests.cs
    CachingRegistrationTests.cs
    StorageRegistrationTests.cs
    QuotaRegistrationTests.cs
    PersistenceRegistrationTests.cs
    WebhooksConformanceTests.cs
    DurableEventingConformanceTests.cs
    BillingConformanceTests.cs
    ServiceReplacementTests.cs
    OptInBoundaryTests.cs
    HealthCheckTests.cs
    CancellationTests.cs
    FailureClassificationTests.cs
    EndToEndHostTests.cs
```

The conformance project is intentionally **not** added to `Platform.sln`. It is
restored, built, and tested by `scripts/conformance.sh`, which packs the
platform projects first so the fixture only sees the published artifacts.

## Running the suite

```bash
./scripts/conformance.sh
```

The script:

1. Packs every platform project into
   `tests/Platform.ConsumerConformance/.local-feed/` using the documented
   `VersionPrefix`.
2. Restores the conformance project against the local feed and the public
   `nuget.org` source.
3. Builds and tests the conformance project in `Release`.
4. Writes per-step logs to `tests/Platform.ConsumerConformance/.logs/`.

Any failure is reported with the exact failed command and the next action so
CI can distinguish a source failure from an environment blocker. The script
returns exit code `0` only when every step succeeds.

## What the suite verifies

- **Shared-contract envelopes** — `ContractEnvelopeConformanceTests` validates
  canonical fixtures for identity subject, permission, tenant, audit, Gate
  result, and release evidence contracts. Compatible documents are accepted;
  missing required fields or incompatible status vocabularies fail with the
  contract and field identified. Fixtures are local JSON files consumed without
  any runtime dependency on a contract repository.
- **Package feed** — The conformance project consumes platform packages
  exclusively via `<PackageReference>`; no `<ProjectReference>` to platform
  projects. `Platform.Architecture.Tests` enforces the same boundary for
  production projects.
- **Registration** — Each platform `AddPlatformXxx` extension registers its
  expected services, options, and clock. The fixture asserts both
  presence and type.
- **Replacement** — Consumers can register their own implementation
  (`IRateLimiter`, `IMailService`, `IIdempotencyStore`,
  `IObjectStorage`, `IQuotaStore`, …) and the platform default does not
  hijack the contract.
- **Health and readiness** — The platform liveness health check responds
  `Healthy` via `TestServer`, additional checks can be registered, and the
  rate-limit backend status provider reports `memory` availability.
- **Failure classification** — `ProviderFailureClassifier` returns the
  documented category for every exception type and the safe message never
  carries the original exception text.
- **Tenant isolation, opt-in boundaries, cancellation, redaction, and
  durable eventing** are covered by the in-memory stores, the SSRF
  validator, the in-process event bus, and the HMAC signature verifier.
- **End-to-end host** — A minimal `WebApplication` built entirely from
  packages reports the documented `ProblemDetails` for validation errors,
  sanitizes unknown failures to a generic `500`, and echoes the correlation
  identifier on the response.

## Boundaries

- Production projects must not reference the consumer conformance project
  (enforced by `Platform.Architecture.Tests`).
- The fixture must not embed a `<ProjectReference>` to a platform project
  (asserted in `PackageFeedVerificationTests` and re-asserted by
  `Platform.Architecture.Tests`).
- The local feed directory is generated and excluded from source control.

## Optional Docker / provider integration

The default suite runs in-process and on the local file system. The
`scripts/conformance.sh` wrapper intentionally fails loudly when a
dependency is missing (for example a missing local NuGet feed) so CI can
distinguish a source failure from an environment blocker. Adopters who
require Docker, Redis, PostgreSQL, or external provider tests can add
explicit gates inside the same fixture by:

- packaging a new `tests/Platform.ConsumerConformance.Integration`
  project that is opt-in via an environment flag;
- invoking that project from a separate CI job that provisions the
  required Docker images and credentials.

The architecture and conformance projects stay green even when those
external services are unavailable.

## Rollback

To disable the conformance gate without touching the platform packages:

1. Remove `scripts/conformance.sh` from CI.
2. Delete the `tests/Platform.ConsumerConformance/` directory.
3. Remove the new architecture tests in
   `tests/Platform.Architecture.Tests/DependencyDirectionTests.cs` that
   reference the conformance project.

No platform package or production code is affected.
