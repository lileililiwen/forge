using System.Text;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Platform.Core.Time;

namespace Platform.Testing.AspNetCore;

/// <summary>
/// Configuration callback used by <see cref="PlatformTestWebApplicationFactory{TEntryPoint}"/>.
/// The platform passes the service collection and configuration builder
/// so callers can swap services for fakes or extend the configuration
/// surface.
/// </summary>
public delegate void PlatformTestHostConfiguration(IServiceCollection services, IConfigurationBuilder configuration);

/// <summary>
/// Scope used when a factory materialises a temporary directory.
/// </summary>
public enum PlatformTestTemporaryDirectoryScope
{
    /// <summary>
    /// Every factory gets its own directory. This is the default and is safe
    /// under xUnit, which runs test collections in parallel.
    /// </summary>
    PerFactory = 0,

    /// <summary>
    /// Every factory that opts in shares one process-wide directory, created on
    /// first use and removed once the last participating factory is disposed.
    /// Opt in only when sharing the contents is deliberate.
    /// </summary>
    ProcessWide = 1,
}

/// <summary>
/// Test host factory that boots the <em>consumer's</em> real ASP.NET Core
/// application over an in-memory <c>TestServer</c>.
/// <para>
/// The factory never constructs an application of its own. It derives from
/// <see cref="WebApplicationFactory{TEntryPoint}"/>, so the inherited seam —
/// <c>ConfigureWebHost</c>,
/// <see cref="WebApplicationFactory{TEntryPoint}.WithWebHostBuilder" />,
/// <see cref="CreateClient()" />, <see cref="WebApplicationFactory{TEntryPoint}.Services" /> and <see cref="WebApplicationFactory{TEntryPoint}.Server" /> —
/// is available without further wiring, and every configuration callback runs
/// against the consumer's host after the consumer's own service registrations.
/// </para>
/// <para>
/// <strong>One host per instance.</strong> Every configuration member on this type
/// mutates <em>this</em> factory, and the base class builds exactly one host from
/// it. <see cref="WebApplicationFactory{TEntryPoint}.WithWebHostBuilder" /> is inherited for
/// compatibility but is deliberately <em>not</em> the supported way to configure a
/// scenario: each call derives a new factory and therefore triggers another host
/// build. Configure before the first access instead. Because of this, a scenario
/// permutation is one factory configured differently per test, not one derived
/// class per configuration key.
/// </para>
/// <para>
/// The generic argument is the application's entry point: the global
/// <c>Program</c> of a top-level-statements app, a namespaced <c>Program</c>, or a
/// public marker type declared by the application. C# has no default type
/// parameter for classes, so the argument is always required; that is what makes a
/// non-<c>Program</c> marker usable.
/// </para>
/// <para>
/// Tests should construct the factory once per scenario and dispose it with
/// <c>await using</c>.
/// </para>
/// </summary>
/// <typeparam name="TEntryPoint">The consumer application's entry point type.</typeparam>
public class PlatformTestWebApplicationFactory<TEntryPoint> : WebApplicationFactory<TEntryPoint>
    where TEntryPoint : class
{
    private const string TemporaryDirectoryRootName = "platform-test-web-factory";
    private const string SharedTemporaryDirectoryLeaf = "shared";

    /// <summary>
    /// Distinctive fragments of <see cref="WebApplicationFactory{TEntryPoint}" />'s own
    /// entry-point resolution failures. Matching on them keeps an application that
    /// throws during startup from being misreported as a bad entry point.
    /// </summary>
    private static readonly string[] EntryPointResolutionFailureMarkers =
    [
        "does not belong to an assembly with an entry point",
        "exited without ever building an IHost",
        "A factory method for runtime host creation",
    ];

    private static readonly object SharedDirectoryGate = new();
    private static int _sharedDirectoryUsers;
    private static string? _sharedDirectoryPath;

    private readonly List<PlatformTestHostConfiguration> _configurations = new();
    private readonly List<Action<IWebHostBuilder>> _hostConfigurations = new();
    private readonly Dictionary<string, string?> _inMemoryConfiguration = new(StringComparer.OrdinalIgnoreCase);
    private readonly List<Action<IServiceCollection>> _serviceOperations = new();
    private readonly List<Func<ValueTask>> _disposeActions = new();

    private string _environmentName = PlatformTestEnvironments.Testing;
    private IConfigurationBuilder? _applicationConfiguration;
    private bool _hostCreationStarted;
    private bool _teardownCompleted;
    private string? _temporaryDirectory;
    private PlatformTestTemporaryDirectoryScope _temporaryDirectoryScope;

    /// <summary>
    /// Gets the environment name the consumer application is booted with.
    /// </summary>
    public string EnvironmentName => _environmentName;

    /// <summary>
    /// Gets the directory created by
    /// <see cref="UseTemporaryDirectory(PlatformTestTemporaryDirectoryScope)"/>.
    /// </summary>
    /// <exception cref="InvalidOperationException">
    /// Thrown when <see cref="UseTemporaryDirectory(PlatformTestTemporaryDirectoryScope)"/>
    /// has not been called.
    /// </exception>
    public string TemporaryDirectory => _temporaryDirectory
        ?? throw new InvalidOperationException(
            "No temporary directory has been requested. Call UseTemporaryDirectory() before reading TemporaryDirectory.");

    /// <summary>
    /// Selects the environment the consumer application is booted with. Any
    /// non-empty name is honoured, so a repository can pin its own environment
    /// name; the consumer's <c>appsettings.&lt;Environment&gt;.json</c> is applied.
    /// Defaults to <see cref="PlatformTestEnvironments.Testing"/> when never called.
    /// </summary>
    /// <param name="environmentName">The environment name to boot the application with.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentException">The name is <see langword="null" />, empty or whitespace.</exception>
    /// <exception cref="InvalidOperationException">The host has already been created.</exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> WithEnvironment(string environmentName)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(environmentName);
        GuardHostNotStarted(nameof(WithEnvironment));
        _environmentName = environmentName;
        return this;
    }

    /// <summary>
    /// Adds an in-memory configuration value that outranks the consumer's own
    /// configuration sources.
    /// </summary>
    /// <param name="key">Configuration key.</param>
    /// <param name="value">Configuration value.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentException">The key is <see langword="null" />, empty or whitespace.</exception>
    /// <exception cref="InvalidOperationException">The host has already been created.</exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> WithConfiguration(string key, string? value)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(key);
        GuardHostNotStarted(nameof(WithConfiguration));
        _inMemoryConfiguration[key] = value;
        return this;
    }

    /// <summary>
    /// Registers a configuration callback that runs against the consumer's host
    /// after the application's own service registrations, so a service the
    /// application registered can be inspected and replaced.
    /// </summary>
    /// <param name="configuration">The callback to run during host configuration.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="configuration" /> is <see langword="null" />.</exception>
    /// <exception cref="InvalidOperationException">The host has already been created.</exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> ConfigureTestServices(
        PlatformTestHostConfiguration configuration)
    {
        ArgumentNullException.ThrowIfNull(configuration);
        GuardHostNotStarted(nameof(ConfigureTestServices));
        _configurations.Add(configuration);
        return this;
    }

    /// <summary>
    /// Registers a host configuration callback against <em>this</em> factory. This
    /// is the same seam as the inherited
    /// <c>ConfigureWebHost</c>,
    /// exposed as a fluent call so a scenario needs no derived class. Callbacks run
    /// in registration order, after the application's own service registrations.
    /// <para>
    /// Prefer this over <see cref="WebApplicationFactory{TEntryPoint}.WithWebHostBuilder" />:
    /// this mutates the current factory and keeps the one-host-per-instance
    /// guarantee, whereas <c>WithWebHostBuilder</c> derives a second factory and
    /// builds a second host.
    /// </para>
    /// </summary>
    /// <param name="configure">The callback to run during host configuration.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="configure" /> is <see langword="null" />.</exception>
    /// <exception cref="InvalidOperationException">The host has already been created.</exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> ConfigureWebHost(Action<IWebHostBuilder> configure)
    {
        ArgumentNullException.ThrowIfNull(configure);
        GuardHostNotStarted(nameof(ConfigureWebHost));
        _hostConfigurations.Add(configure);
        return this;
    }

    /// <summary>
    /// Replaces every registration of <typeparamref name="TService"/> with a single
    /// instance supplied by the test.
    /// </summary>
    /// <typeparam name="TService">The service contract to replace.</typeparam>
    /// <param name="instance">The instance the application must resolve.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="instance" /> is <see langword="null" />.</exception>
    /// <exception cref="InvalidOperationException">
    /// The host has already been created, or the application never registered
    /// <typeparamref name="TService"/>.
    /// </exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> ReplaceService<TService>(TService instance)
        where TService : class
    {
        ArgumentNullException.ThrowIfNull(instance);
        GuardHostNotStarted(nameof(ReplaceService));
        _serviceOperations.Add(services =>
        {
            EnsureRegistered<TService>(services);
            services.RemoveAll<TService>();
            services.AddSingleton(instance);
        });
        return this;
    }

    /// <summary>
    /// Replaces every registration of <typeparamref name="TService"/> with a single
    /// <typeparamref name="TImplementation"/> registration resolved from the
    /// container.
    /// </summary>
    /// <typeparam name="TService">The service contract to replace.</typeparam>
    /// <typeparam name="TImplementation">The implementation the application must resolve.</typeparam>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="InvalidOperationException">
    /// The host has already been created, or the application never registered
    /// <typeparamref name="TService"/>.
    /// </exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> ReplaceService<TService, TImplementation>()
        where TService : class
        where TImplementation : class, TService
    {
        GuardHostNotStarted(nameof(ReplaceService));
        _serviceOperations.Add(services =>
        {
            EnsureRegistered<TService>(services);
            services.RemoveAll<TService>();
            services.AddSingleton<TService, TImplementation>();
        });
        return this;
    }

    /// <summary>
    /// Wraps the application's own <typeparamref name="TService"/> implementation.
    /// The original instance is resolved from the container and handed to
    /// <paramref name="decorate" />, so it stays reachable inside the wrapper the
    /// callback returns.
    /// </summary>
    /// <typeparam name="TService">The service contract to decorate.</typeparam>
    /// <typeparam name="TWrapper">
    /// The concrete wrapper type <paramref name="decorate" /> is expected to return.
    /// It names the wrapper at the call site so the shape is readable.
    /// </typeparam>
    /// <param name="decorate">Receives the original instance and returns the replacement.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="decorate" /> is <see langword="null" />.</exception>
    /// <exception cref="InvalidOperationException">
    /// The host has already been created, or the application never registered
    /// <typeparamref name="TService"/>.
    /// </exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> DecorateService<TService, TWrapper>(
        Func<TService, TService> decorate)
        where TService : class
        where TWrapper : class
    {
        ArgumentNullException.ThrowIfNull(decorate);
        GuardHostNotStarted(nameof(DecorateService));
        _serviceOperations.Add(services =>
        {
            EnsureRegistered<TService>(services);
            var descriptors = services.Where(descriptor => descriptor.ServiceType == typeof(TService)).ToArray();
            var original = descriptors[^1];
            services.RemoveAll<TService>();
            services.Add(new ServiceDescriptor(
                typeof(TService),
                provider => decorate(ResolveOriginal<TService>(provider, original)),
                original.Lifetime));
        });
        return this;
    }

    /// <summary>
    /// Makes the booted application resolve the supplied <see cref="IClock" />. The
    /// clock is registered whether or not the application registered one, so a
    /// consumer that does not use the platform abstraction can still be given a
    /// deterministic clock.
    /// </summary>
    /// <param name="clock">The clock the application must resolve.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="clock" /> is <see langword="null" />.</exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> WithClock(IClock clock)
    {
        ArgumentNullException.ThrowIfNull(clock);
        return UpsertService<IClock>(clock);
    }

    /// <summary>
    /// Makes the booted application resolve the supplied
    /// <see cref="TimeProvider" />, for consumers bound to the BCL abstraction
    /// rather than <see cref="IClock" />. Registered whether or not the application
    /// registered one.
    /// </summary>
    /// <param name="timeProvider">The time provider the application must resolve.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="timeProvider" /> is <see langword="null" />.</exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> WithClock(TimeProvider timeProvider)
    {
        ArgumentNullException.ThrowIfNull(timeProvider);
        return UpsertService(timeProvider);
    }

    /// <summary>
    /// Registers a cleanup action to run after the host has stopped. Actions run in
    /// the reverse of their registration order; a failing action does not prevent
    /// the remaining actions or host disposal, and the failure is surfaced to the
    /// caller of <see cref="DisposeAsync" />.
    /// </summary>
    /// <param name="action">The cleanup action.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="action" /> is <see langword="null" />.</exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> OnDispose(Action action)
    {
        ArgumentNullException.ThrowIfNull(action);
        return OnDispose(() =>
        {
            action();
            return ValueTask.CompletedTask;
        });
    }

    /// <summary>
    /// Registers an asynchronous cleanup action to run after the host has stopped.
    /// Actions run in the reverse of their registration order; a failing action does
    /// not prevent the remaining actions or host disposal, and the failure is
    /// surfaced to the caller of <see cref="DisposeAsync" />.
    /// </summary>
    /// <param name="action">The cleanup action.</param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="action" /> is <see langword="null" />.</exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> OnDispose(Func<ValueTask> action)
    {
        ArgumentNullException.ThrowIfNull(action);
        GuardHostNotStarted(nameof(OnDispose));
        _disposeActions.Add(action);
        return this;
    }

    /// <summary>
    /// Creates a unique, writable directory beneath the operating system's
    /// temporary root and exposes it through <see cref="TemporaryDirectory" />. The
    /// directory is never used as a content root and the process working directory
    /// is not changed. It is removed when the factory is disposed.
    /// </summary>
    /// <param name="scope">
    /// <see cref="PlatformTestTemporaryDirectoryScope.PerFactory" /> (default) gives
    /// each factory its own directory.
    /// <see cref="PlatformTestTemporaryDirectoryScope.ProcessWide" /> opts into one
    /// shared process-wide directory, created on first use and removed once the last
    /// participating factory is disposed.
    /// </param>
    /// <returns>This factory, for chaining.</returns>
    /// <exception cref="InvalidOperationException">The host has already been created.</exception>
    public PlatformTestWebApplicationFactory<TEntryPoint> UseTemporaryDirectory(
        PlatformTestTemporaryDirectoryScope scope = PlatformTestTemporaryDirectoryScope.PerFactory)
    {
        GuardHostNotStarted(nameof(UseTemporaryDirectory));
        ReleaseTemporaryDirectory();
        _temporaryDirectoryScope = scope;
        _temporaryDirectory = scope == PlatformTestTemporaryDirectoryScope.ProcessWide
            ? AcquireSharedTemporaryDirectory()
            : CreateFreshTemporaryDirectory();
        return this;
    }

    /// <inheritdoc />
    public override async ValueTask DisposeAsync()
    {
        if (_teardownCompleted)
        {
            await base.DisposeAsync().ConfigureAwait(false);
            GC.SuppressFinalize(this);
            return;
        }

        _teardownCompleted = true;
        Exception? hostFailure = null;
        try
        {
            // The host stops and is disposed before any teardown action runs.
            await base.DisposeAsync().ConfigureAwait(false);
        }
#pragma warning disable CA1031 // A failing host disposal must not skip the registered teardown actions.
        catch (Exception ex)
        {
            hostFailure = ex;
        }
#pragma warning restore CA1031

        var failures = new List<Exception>();
        if (hostFailure is not null)
        {
            failures.Add(hostFailure);
        }

        for (var index = _disposeActions.Count - 1; index >= 0; index--)
        {
            try
            {
                await _disposeActions[index]().ConfigureAwait(false);
            }
#pragma warning disable CA1031 // A failing teardown action must not abort the remaining actions or host disposal.
            catch (Exception ex)
            {
                failures.Add(ex);
            }
#pragma warning restore CA1031
        }

        ReleaseTemporaryDirectory();
        GC.SuppressFinalize(this);

        if (failures.Count > 0)
        {
            throw new AggregateException(
                failures.Count == 1
                    ? "Disposing the platform test web application factory failed."
                    : $"Disposing the platform test web application factory failed with {failures.Count} failure(s).",
                failures);
        }
    }

    /// <summary>
    /// Gets a test client wired to the in-memory server.
    /// </summary>
    /// <returns>An <see cref="HttpClient" /> bound to the consumer application.</returns>
    /// <exception cref="InvalidOperationException">
    /// The entry point cannot boot an application. The diagnostic names the type,
    /// its declaring assembly and the two supported fixes.
    /// </exception>
    public new HttpClient CreateClient() => WithEntryPointDiagnostic(base.CreateClient);

    /// <summary>
    /// Gets a test client wired to the in-memory server using explicit options.
    /// </summary>
    /// <param name="options">The client options to apply.</param>
    /// <returns>An <see cref="HttpClient" /> bound to the consumer application.</returns>
    public new HttpClient CreateClient(WebApplicationFactoryClientOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        return WithEntryPointDiagnostic(() => base.CreateClient(options));
    }

    /// <summary>
    /// Gets the <see cref="TestServer" /> hosting the consumer application.
    /// </summary>
    public new TestServer Server => WithEntryPointDiagnostic(() => base.Server);

    /// <inheritdoc />
    public override IServiceProvider Services => WithEntryPointDiagnostic(() => base.Services);

    /// <summary>
    /// Applies the factory's accumulated configuration to the consumer's host. Every
    /// callback runs here, which is the point at which the application's own service
    /// registrations exist and can be replaced.
    /// </summary>
    /// <param name="builder">The builder for the consumer's host.</param>
    protected override void ConfigureWebHost(IWebHostBuilder builder)
    {
        ArgumentNullException.ThrowIfNull(builder);
        _hostCreationStarted = true;

        // Applied on both the host and the web host so IHostEnvironment and
        // IWebHostEnvironment report the same name, including on the deferred
        // (minimal-hosting) bootstrap where no CreateHostBuilder exists.
        builder.UseEnvironment(_environmentName);

        if (_inMemoryConfiguration.Count > 0)
        {
            var overlay = new Dictionary<string, string?>(_inMemoryConfiguration, StringComparer.OrdinalIgnoreCase);
            builder.ConfigureAppConfiguration((_, configuration) =>
                configuration.AddInMemoryCollection(overlay));
        }

        foreach (var configure in _hostConfigurations)
        {
            configure(builder);
        }

        if (_configurations.Count == 0 && _serviceOperations.Count == 0)
        {
            return;
        }

        var configurations = _configurations.ToArray();
        var serviceOperations = _serviceOperations.ToArray();

        if (configurations.Length > 0)
        {
            // Capture the application's own configuration builder. Under minimal
            // hosting it is a live configuration manager, so sources the callback
            // adds below are honoured by the booted application.
            builder.ConfigureAppConfiguration((_, configuration) => _applicationConfiguration = configuration);
        }

        if (configurations.Length > 0 || serviceOperations.Length > 0)
        {
            builder.ConfigureServices(services =>
            {
                if (configurations.Length > 0)
                {
                    var configuration = _applicationConfiguration ?? new ConfigurationBuilder();
                    foreach (var configure in configurations)
                    {
                        configure(services, configuration);
                    }
                }

                foreach (var operation in serviceOperations)
                {
                    operation(services);
                }
            });
        }
    }

    /// <inheritdoc />
    protected override IHost CreateHost(IHostBuilder builder)
    {
        ArgumentNullException.ThrowIfNull(builder);
        return WithEntryPointDiagnostic(() => base.CreateHost(builder.UseEnvironment(_environmentName)));
    }

    /// <summary>
    /// Runs an action that boots the host and replaces the base class's opaque
    /// entry-point failures with a diagnostic that names the type, its declaring
    /// assembly and the two supported fixes. Only the base class's own
    /// entry-point resolution failures are translated: an application that throws
    /// while starting still surfaces its own exception unchanged.
    /// </summary>
    private static T WithEntryPointDiagnostic<T>(Func<T> action)
    {
        try
        {
            return action();
        }
        catch (InvalidOperationException ex) when (IsEntryPointResolutionFailure(ex))
        {
            throw new InvalidOperationException(BuildEntryPointDiagnostic(ex), ex);
        }
    }

    private static bool IsEntryPointResolutionFailure(InvalidOperationException exception)
    {
        foreach (var marker in EntryPointResolutionFailureMarkers)
        {
            if (exception.Message.Contains(marker, StringComparison.Ordinal))
            {
                return true;
            }
        }

        return false;
    }

    private PlatformTestWebApplicationFactory<TEntryPoint> UpsertService<TService>(TService instance)
        where TService : class
    {
        GuardHostNotStarted(nameof(WithClock));
        _serviceOperations.Add(services =>
        {
            services.RemoveAll<TService>();
            services.AddSingleton(instance);
        });
        return this;
    }

    private static TService ResolveOriginal<TService>(IServiceProvider provider, ServiceDescriptor descriptor)
        where TService : class
    {
        if (descriptor.ImplementationInstance is TService instance)
        {
            return instance;
        }

        if (descriptor.ImplementationFactory is not null)
        {
            return (TService)descriptor.ImplementationFactory(provider);
        }

        if (descriptor.ImplementationType is not null)
        {
            return (TService)ActivatorUtilities.CreateInstance(provider, descriptor.ImplementationType);
        }

        throw new InvalidOperationException(
            $"The registration for '{typeof(TService).FullName}' exposes no implementation to decorate.");
    }

    private static void EnsureRegistered<TService>(IServiceCollection services)
    {
        foreach (var descriptor in services)
        {
            if (descriptor.ServiceType == typeof(TService))
            {
                return;
            }
        }

        throw new InvalidOperationException(
            $"Cannot replace '{typeof(TService).FullName}': the application under test never registered it. " +
            "ReplaceService only works on a service the application itself registered — register the service in the " +
            "application, or use ConfigureWebHost(b => b.ConfigureServices(...)) to add a service the factory cannot see.");
    }

    private void GuardHostNotStarted(string operation)
    {
        if (_hostCreationStarted)
        {
            throw new InvalidOperationException(
                $"Cannot call {operation} after the host has been created. Configure the factory before its first " +
                "access to Services, Server or CreateClient(). Calling WithWebHostBuilder would build a second host, " +
                "so it is not offered as a way to configure a scenario after the fact.");
        }
    }

    /// <summary>
    /// Builds the named diagnostic for an entry point the factory cannot boot. It
    /// names the type, the assembly that declares it, and both supported fixes.
    /// </summary>
    private static string BuildEntryPointDiagnostic(Exception? inner)
    {
        var entryPointType = typeof(TEntryPoint);
        return new StringBuilder()
            .Append("PlatformTestWebApplicationFactory<")
            .Append(entryPointType.FullName)
            .Append("> cannot boot an application. No host builder was found for entry point type '")
            .Append(entryPointType.FullName)
            .Append("', declared in assembly '")
            .Append(entryPointType.Assembly.GetName().Name ?? "<unknown>")
            .Append("'. ")
            .Append("The type is either not an application entry point, or it is not visible from this test ")
            .Append("assembly: a hand-written 'Program' that is internal to its own assembly cannot be named here. ")
            .Append("Fix it by adding [assembly: InternalsVisibleTo(\"<test assembly>\")] to the application, or by ")
            .Append("declaring a public marker type in the application (for example 'public sealed class ")
            .Append("ProgramMarker { }') and passing that marker as the entry point. The factory never falls back to ")
            .Append("an application of its own.")
            .Append(inner is null ? string.Empty : " Underlying failure: " + inner.Message)
            .ToString();
    }

    private static string CreateFreshTemporaryDirectory()
    {
        var path = Path.Combine(
            Path.GetTempPath(),
            TemporaryDirectoryRootName,
            Guid.NewGuid().ToString("N"));
        RecreateDirectory(path);
        return path;
    }

    private static string AcquireSharedTemporaryDirectory()
    {
        lock (SharedDirectoryGate)
        {
            _sharedDirectoryPath ??= Path.Combine(
                Path.GetTempPath(),
                TemporaryDirectoryRootName,
                SharedTemporaryDirectoryLeaf);
            _sharedDirectoryUsers++;
            if (_sharedDirectoryUsers == 1)
            {
                RecreateDirectory(_sharedDirectoryPath);
            }

            return _sharedDirectoryPath;
        }
    }

    private void ReleaseTemporaryDirectory()
    {
        if (_temporaryDirectory is null)
        {
            return;
        }

        if (_temporaryDirectoryScope == PlatformTestTemporaryDirectoryScope.ProcessWide)
        {
            lock (SharedDirectoryGate)
            {
                _sharedDirectoryUsers = Math.Max(0, _sharedDirectoryUsers - 1);
                if (_sharedDirectoryUsers == 0)
                {
                    TryDelete(_sharedDirectoryPath);
                    _sharedDirectoryPath = null;
                }
            }
        }
        else
        {
            TryDelete(_temporaryDirectory);
        }

        _temporaryDirectory = null;
    }

    private static void RecreateDirectory(string path)
    {
        // Replace rather than merge: anything already at the chosen path is not
        // test-owned and must not be reported as such.
        TryDelete(path);
        Directory.CreateDirectory(path);
    }

    private static void TryDelete(string? path)
    {
        if (string.IsNullOrEmpty(path))
        {
            return;
        }

        try
        {
            if (Directory.Exists(path))
            {
                Directory.Delete(path, recursive: true);
            }
        }
        catch (IOException)
        {
        }
        catch (UnauthorizedAccessException)
        {
        }
    }
}

/// <summary>
/// Stable, public environment names used by the testing toolkit.
/// </summary>
public static class PlatformTestEnvironments
{
    /// <summary>
    /// Environment name used by <see cref="PlatformTestWebApplicationFactory{TEntryPoint}"/>
    /// when <c>WithEnvironment</c> is not called.
    /// </summary>
    public const string Testing = "Testing";
}
