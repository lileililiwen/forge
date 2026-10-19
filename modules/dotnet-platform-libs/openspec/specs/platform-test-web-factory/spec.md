# platform-test-web-factory Specification

## Purpose

Provide `PlatformTestWebApplicationFactory<TEntryPoint>`, a shared web test factory that boots
the consumer's real application over `TestServer` from a named entry point, so a repository no
longer duplicates bootstrap, DI graph and middleware pipeline in dozens of locally written
`WebApplicationFactory<T>` subclasses — many of which exist only to express a configuration
permutation. The beneficiary is a consumer writing integration tests against its own host,
including call sites that already use `WithWebHostBuilder` and must keep working unchanged. The
boundary is a test-only, dependency-light surface: exactly one host per factory instance with no
silent fallback and no application of its own, with environment selection, service replacement,
clock injection, teardown actions and an isolated temporary directory expressed without
subclassing, and no provider fixture or ORM dependency entering the package.
## Requirements
### Requirement: The shared web test factory SHALL boot the consumer's real application

`Platform.Testing.AspNetCore` SHALL provide
`PlatformTestWebApplicationFactory<TEntryPoint> where TEntryPoint : class`, which
SHALL be a `WebApplicationFactory<TEntryPoint>` and SHALL host the consumer's
own application over `TestServer` using `TEntryPoint` as the entry point. It
SHALL NOT construct a replacement application of its own, and it SHALL NOT fall
back to an application that does not belong to the consumer.

#### Scenario: Entry point is the application's Program

- **WHEN** a test constructs `PlatformTestWebApplicationFactory<Program>` for an
  application whose `Program` type is discoverable from the test project
- **THEN** the factory boots that application, and a request through
  `CreateClient()` reaches a route declared by the application itself
- **AND** no route exists that the factory added on the application's behalf

#### Scenario: Entry point is a dedicated public marker

- **WHEN** a test constructs the factory with an application-specific marker
  type such as `Stylify.Web.ProgramMarker` or
  `SomadaNote.Web.WebAssemblyMarker` that is `public` and reachable from the test
  project
- **THEN** the factory boots that application through the marker, exactly as it
  does for a `Program` type
- **AND** the factory does not require any specific marker type from any specific
  package

#### Scenario: Entry point is a Program type in a namespace

- **WHEN** a test constructs the factory with a `Program` type that is declared
  inside a namespace such as `Arivio.Api.Program` or `HistoryKB.Web.Program`
- **THEN** the factory resolves that namespaced type as the entry point
- **AND** it does not fall back to a global `Program` type

#### Scenario: Entry point type is not visible from the test project

- **WHEN** a test constructs the factory with an entry point type that is
  `internal` to the application assembly and the test assembly has no
  `InternalsVisibleTo` access to it
- **THEN** the factory fails with a named diagnostic that identifies the entry
  point type and the assembly that declares it
- **AND** the diagnostic names `InternalsVisibleTo` or a public marker type as
  the remedy
- **AND** the factory does not boot an application of its own as a fallback

#### Scenario: Entry point type exposes no host builder

- **WHEN** a test constructs the factory with a type that is not an application
  entry point and from which no host builder can be resolved
- **THEN** the factory fails with a named diagnostic that identifies the supplied
  type
- **AND** the factory does not boot an application of its own as a fallback

#### Scenario: The application fails during startup

- **WHEN** the consumer's application throws while starting under the factory
- **THEN** the original startup failure reaches the test unchanged
- **AND** the factory does not convert it into a successful host

### Requirement: Host configuration SHALL run against the consumer's host

The factory SHALL support the `ConfigureWebHost(Action<IWebHostBuilder>)` seam
inherited from `WebApplicationFactory<TEntryPoint>`, and every configuration
callback SHALL be applied to the consumer's host rather than to a host the
factory owns. Callbacks SHALL execute after the application's own service
registrations, so that a service registered by the application can be inspected
and replaced.

#### Scenario: Configuration overlay through UseSetting

- **WHEN** a test calls `ConfigureWebHost(builder => builder.UseSetting(key, value))`
  and then resolves the booted application
- **THEN** the value is visible to the application through its configuration

#### Scenario: In-memory configuration outranks the application's own sources

- **WHEN** a test adds an in-memory collection through
  `ConfigureAppConfiguration` for a key that the application also defines in
  `appsettings.json`
- **THEN** the application observes the value supplied by the test

#### Scenario: Logging, services and hosted-service configuration

- **WHEN** a test applies `ConfigureLogging` or `ConfigureServices` through
  `ConfigureWebHost`
- **THEN** the consumer's logging pipeline and service registrations reflect the
  test's configuration
- **AND** background workers and hosted services added by the application can be
  removed or replaced through the same seam

#### Scenario: The consumer's static and application assets are found

- **WHEN** a test boots a consumer application that serves static files or Razor
  views, and requests such an asset
- **THEN** the asset is resolved from the consumer application's own output
  directory
- **AND** the content root is not pinned to the test assembly's base directory
  in a way that makes the consumer's assets unreachable

#### Scenario: A configuration callback throws

- **WHEN** a callback passed to `ConfigureWebHost` throws
- **THEN** the exception propagates to the caller that triggered the host build
- **AND** no partially configured host is left behind and no application of the
  factory's own is started

#### Scenario: Configuration applied after the host has been created

- **WHEN** a test calls any configuration or mutation method after the host has
  already been created by an earlier access
- **THEN** the factory fails with a named `InvalidOperationException` that
  directs the caller to configure the factory before first access or to use
  `WithWebHostBuilder`

### Requirement: The factory SHALL allow the test environment to be selected

The factory SHALL provide `WithEnvironment(string environmentName)`. The
environment SHALL default to `PlatformTestEnvironments.Testing` when
`WithEnvironment` is not called, and SHALL be applied to the booted application
so that the consumer's environment-specific configuration is honoured.

#### Scenario: Default environment

- **WHEN** a test constructs the factory without calling `WithEnvironment`
- **THEN** the booted application reports `PlatformTestEnvironments.Testing` as
  its environment name

#### Scenario: Explicit environment is applied with the consumer's own configuration

- **WHEN** a test calls `WithEnvironment("Staging")` against an application that
  ships an `appsettings.Staging.json`
- **THEN** the booted application reports `Staging` as its environment name
- **AND** values from the application's `appsettings.Staging.json` are applied

#### Scenario: Missing or empty environment name

- **WHEN** a test calls `WithEnvironment` with `null`, an empty string, or a
  whitespace-only string
- **THEN** the call throws `ArgumentException` at the call site
- **AND** no host has been created

#### Scenario: Environment selected after the host has been created

- **WHEN** a test calls `WithEnvironment` after the host has already been created
- **THEN** the factory fails with the same named mutation error as any other
  post-creation mutation

### Requirement: The factory SHALL replace and decorate registered services

The factory SHALL provide `ReplaceService<TService>(TService instance)`,
`ReplaceService<TService, TImpl>()`, and
`DecorateService<TService, TWrapper>(Func<TService, TService> decorate)`. These
operations SHALL apply to the consumer's own service registrations, SHALL remove
all existing registrations of the target service rather than appending to them,
and SHALL fail loudly when the target service is not registered.

#### Scenario: Replace a service the application registered

- **WHEN** a test calls `ReplaceService<IEmailSender>(fake)` for a service the
  application registered, and then resolves `IEmailSender` from the booted
  application
- **THEN** the resolved instance is the supplied fake and no registration of the
  application's own implementation remains effective

#### Scenario: Replace a service the application registered more than once

- **WHEN** the application registered `TService` more than once, and a test calls
  `ReplaceService<TService>(instance)`
- **THEN** every prior registration of `TService` is removed
- **AND** resolving `TService` yields the single supplied instance

#### Scenario: Replace a service with a type

- **WHEN** a test calls `ReplaceService<TService, TImplementation>()` for a
  service the application registered
- **THEN** resolving `TService` yields the registered `TImplementation`

#### Scenario: Decorate a service and retain the original

- **WHEN** a test calls `DecorateService<TService, TWrapper>(decorate)` for a
  service the application registered, and `decorate` returns a wrapper that holds
  the service it received
- **THEN** resolving `TService` yields the wrapper
- **AND** the original application-provided implementation remains reachable
  through the wrapper

#### Scenario: Replace a service that was never registered

- **WHEN** a test calls `ReplaceService<TService>` for a `TService` that the
  application did not register
- **THEN** the factory fails with a named `InvalidOperationException`
- **AND** the failure is not reported as a successful test setup

#### Scenario: Null service instance

- **WHEN** a test passes `null` to `ReplaceService<TService>(instance)`
- **THEN** the call throws `ArgumentNullException` at the call site

### Requirement: The factory SHALL inject a deterministic clock

The factory SHALL provide `WithClock(IClock clock)`, bound to the
`Platform.Core.IClock` abstraction already exposed by the platform, so that the
booted application resolves the supplied clock instead of its own. The factory
SHALL NOT add a package reference in order to do so.

#### Scenario: Supplied clock is resolved by the application

- **WHEN** a test calls `WithClock(fixedClock)` for an application that resolves
  `IClock` from its service provider
- **THEN** the application observes the supplied clock's time
- **AND** repeated reads return the same instant, so timing assertions are
  deterministic

#### Scenario: No clock is supplied

- **WHEN** a test does not call `WithClock`
- **THEN** the booted application resolves whatever clock it registers itself,
  unchanged

#### Scenario: Null clock

- **WHEN** a test passes `null` to `WithClock`
- **THEN** the call throws `ArgumentNullException` at the call site

### Requirement: The factory SHALL register provider-agnostic teardown actions

The factory SHALL provide `OnDispose(Action action)` and
`OnDispose(Func<ValueTask> action)` so that a test, or a future external-resource
fixture built on the factory, can register cleanup. Actions SHALL run only after
the host has stopped, SHALL run in reverse registration order, and a failing
action SHALL NOT prevent the remaining actions or the host from being disposed.

#### Scenario: Teardown actions run after the host stops

- **WHEN** a test registers two teardown actions and then disposes the factory
- **THEN** the host has already stopped when the first action runs
- **AND** the actions run in the reverse of their registration order

#### Scenario: Asynchronous teardown is awaited

- **WHEN** a test registers an asynchronous teardown action and disposes the
  factory
- **THEN** the factory awaits that action to completion before disposal returns

#### Scenario: One teardown action fails

- **WHEN** one registered teardown action throws and others are registered after
  it
- **THEN** the remaining actions still run and the host is still disposed
- **AND** the failure is surfaced to the caller of the disposal rather than
  swallowed

#### Scenario: A future external-resource fixture registers its own cleanup

- **WHEN** a resource provisioned for a test, such as a database or a container,
  is registered for cleanup through `OnDispose`
- **THEN** its cleanup is released by the factory's disposal without the
  factory needing to know what kind of resource it is

#### Scenario: Null teardown action

- **WHEN** a test passes `null` to `OnDispose`
- **THEN** the call throws `ArgumentNullException` at the call site

### Requirement: The factory SHALL provide an isolated temporary directory

The factory SHALL provide `UseTemporaryDirectory()` and a `TemporaryDirectory`
accessor that expose a unique, writable directory beneath the operating system's
temporary root, and SHALL remove that directory when the factory is disposed.
The directory SHALL NOT be used as the content root and SHALL NOT change the
process working directory.

#### Scenario: Temporary directory is unique and writable

- **WHEN** two factories each call `UseTemporaryDirectory`
- **THEN** each reports a distinct, empty, writable path
- **AND** both paths are beneath the operating system's temporary root

#### Scenario: Temporary directory is removed on disposal

- **WHEN** a factory that called `UseTemporaryDirectory` is disposed
- **THEN** the directory it reported no longer exists

#### Scenario: A pre-existing directory at the chosen path

- **WHEN** a path at the chosen location already exists and is not empty
- **THEN** the factory replaces it rather than merging into it
- **AND** the content that was there beforehand is not reported as test-owned

#### Scenario: The temporary directory is not the content root

- **WHEN** a test inspects the booted application after calling
  `UseTemporaryDirectory`
- **THEN** the application's content root is unchanged by that call
- **AND** the process working directory is unchanged

### Requirement: The shared web test factory SHALL build exactly one host per factory instance

The factory SHALL build exactly one host from one instance, and consumer
configuration SHALL be applied to that instance — through the
`ConfigureWebHost` override or the constructor-time configuration members — rather
than by calling `WithWebHostBuilder` on the factory. The factory SHALL NOT derive
or build a second host to express a scenario.

#### Scenario: Repeated access reuses the same host

- **WHEN** a test accesses `Services`, `Server` or `CreateClient()` more than once
  on one factory
- **THEN** the same host and the same service provider are returned
- **AND** the factory's configuration callbacks ran exactly once

#### Scenario: Configuring a scenario does not derive a second factory

- **WHEN** a test configures the factory through `ConfigureWebHost`,
  `ConfigureTestServices`, `WithConfiguration`, `WithEnvironment`,
  `ReplaceService` or `DecorateService`
- **THEN** no additional factory is derived and no second host is built
- **AND** the factory reports no derived factories

#### Scenario: A consumer that subclasses the factory still builds one host

- **WHEN** a consumer overrides `ConfigureWebHost` in a subclass, as the existing
  local factories do
- **THEN** the subclass still builds exactly one host
- **AND** the override composes with the shared configuration

### Requirement: The factory SHALL express configuration permutations without subclassing

The factory SHALL allow one factory type to be parameterized over a scenario, so a
consumer expresses a configuration permutation by configuring the shared factory
differently per scenario rather than by declaring a near-identical derived class per
configuration key. Each configured factory SHALL observe only its own configuration
and service state.

#### Scenario: One factory type covers several scenarios

- **WHEN** a consumer's integration tests previously required a separate
  `WebApplicationFactory<T>` subclass per configuration key, and instead
  configures the shared factory differently per scenario
- **THEN** each scenario boots an application with only its own configuration
  applied
- **AND** no derived class per configuration key is required

#### Scenario: Two differently configured factories stay isolated

- **WHEN** a test creates two factories with different configuration and resolves
  a service from each
- **THEN** each factory's application observes only its own configuration
- **AND** neither factory observes state registered into the other

### Requirement: The shared web test factory SHALL stay test-only and dependency-light

`Platform.Testing.AspNetCore` SHALL remain a test-only package that no production
platform package references, and this change SHALL NOT add a package reference
for a unit-test framework, a mocking framework, an ORM, or a container runtime.
Database isolation and schema bootstrap SHALL remain outside this capability,
which SHALL expose only the generic seams a future provider fixture needs: a
configuration-supplied connection string, a host configuration callback, and a
teardown registration.

#### Scenario: No test framework or ORM reference is added

- **WHEN** the package's declared references are inspected after this change
- **THEN** no unit-test framework, mocking framework, ORM, or container-runtime
  reference is present
- **AND** the generated package manifest is unchanged from before the change

#### Scenario: Production packages still cannot reference it

- **WHEN** the architecture guard inspects every production project's references
- **THEN** any reference to `Platform.Testing.AspNetCore` fails the guard

#### Scenario: Database isolation stays with the consumer for now

- **WHEN** a consumer needs a database with an isolated identity per test, or
  needs schema created or reset between tests
- **THEN** the consumer expresses it through the configuration overlay, the host
  configuration callback and the teardown registration
- **AND** the shared factory provides no database migration, schema-creation or
  reset operation of its own

#### Scenario: A repository whose tests require unavailable infrastructure

- **WHEN** a repository's test suite depends on a database or container that is
  not available in the current environment
- **THEN** that suite is reported as skipped or unverified, and is not reported as
  passing on the basis of this shared factory
