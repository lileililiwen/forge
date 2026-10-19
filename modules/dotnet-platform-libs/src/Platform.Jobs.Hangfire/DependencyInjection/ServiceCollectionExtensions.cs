using Hangfire;
using Hangfire.InMemory;
using Hangfire.PostgreSql;
using Hangfire.PostgreSql.Factories;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.Jobs.Hangfire.DependencyInjection;

/// <summary>
/// Opt-in registration for the Hangfire jobs adapter. The registration
/// never overwrites an application-owned <see cref="IJobDispatcher"/> or
/// <see cref="IRecurringJobRegistry"/>: the adapter implementations are
/// added with <c>TryAdd</c> so consumer registrations made before this call
/// always win. Calling the registration twice is a no-op.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the validated <see cref="HangfireJobsOptions"/>, wires the
    /// application-selected Hangfire storage, the scoped job activator, the
    /// context-capture filter, the Hangfire server, and the platform
    /// dispatcher/registry/health seams. The storage is created from
    /// <see cref="HangfireJobsOptions"/> inside the Hangfire configuration
    /// callback.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The optional configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    /// <exception cref="ArgumentException">The configured options are invalid.</exception>
    public static IServiceCollection AddPlatformHangfireJobs(
        this IServiceCollection services,
        Action<HangfireJobsOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);
        return RegisterHangfireJobs(services, configure, storage: null);
    }

    /// <summary>
    /// Registers the Hangfire adapter using an application-owned
    /// <see cref="JobStorage"/> instance. The provided storage is registered
    /// as the DI singleton so the service provider owns its lifetime, and
    /// is wired into Hangfire's global configuration so the background
    /// server, clients, and the platform dispatcher all use the same
    /// instance. Use this overload when the application or a test fixture
    /// wants explicit storage ownership (for example, to share storage
    /// across hosts, to dispose it deterministically, or to verify storage
    /// disposal in a regression test).
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The optional configuration delegate.</param>
    /// <param name="storage">The application-owned Hangfire storage.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> or <paramref name="storage"/> is <c>null</c>.</exception>
    /// <exception cref="ArgumentException">The configured options are invalid.</exception>
    public static IServiceCollection AddPlatformHangfireJobs(
        this IServiceCollection services,
        Action<HangfireJobsOptions>? configure,
        JobStorage storage)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(storage);
        return RegisterHangfireJobs(services, configure, storage);
    }

    private static IServiceCollection RegisterHangfireJobs(
        IServiceCollection services,
        Action<HangfireJobsOptions>? configure,
        JobStorage? storage)
    {
        if (services.Any(descriptor => descriptor.ServiceType == typeof(HangfireJobsRegistrationMarker)))
        {
            return services;
        }

        Action<HangfireJobsOptions> configureDelegate = configure ?? DefaultConfigure;
        var validated = new HangfireJobsOptions();
        configureDelegate(validated);
        validated.Validate();

        services.AddOptions<HangfireJobsOptions>().Configure(configureDelegate);
        services.AddSingleton(new HangfireJobsRegistrationMarker());

        services.TryAddSingleton<IClock>(_ => new SystemClock());

        if (storage is not null)
        {
            // The application-owned storage wins; registering it explicitly
            // before AddHangfire ensures TryAdd inside AddHangfire does not
            // overwrite the registration with the JobStorage.Current factory.
            services.AddSingleton(storage);
        }

        services.AddHangfire((provider, configuration) =>
        {
            // The storage is created inside the configuration callback so it
            // is constructed after Hangfire has bound the host's log
            // provider; Hangfire resolves JobStorage lazily from
            // JobStorage.Current once the configuration has run. When the
            // application supplied an explicit storage, the explicit
            // registration in the service collection takes precedence over
            // the storage selected from the options.
            configuration.UseStorage(storage ?? CreateStorage(validated));
            configuration.UseActivator(new ScopedJobActivator(provider.GetRequiredService<IServiceScopeFactory>()));
            configuration.UseFilter(new JobContextCaptureFilter(provider));
        });

        services.AddHangfireServer(serverOptions =>
        {
            serverOptions.WorkerCount = validated.WorkerCount;
            serverOptions.Queues = validated.Queues.ToArray();
            serverOptions.SchedulePollingInterval = validated.SchedulePollingInterval;
            serverOptions.HeartbeatInterval = validated.HeartbeatInterval;
        });

        services.TryAddTransient<HangfireJobExecutor>();
        services.TryAddSingleton<IJobDispatcher, HangfireJobDispatcher>();
        services.TryAddSingleton<IRecurringJobRegistry, HangfireRecurringJobRegistry>();
        services.TryAddSingleton<IHealthCheck, HangfireStorageHealthCheck>();

        return services;
    }

    private static void DefaultConfigure(HangfireJobsOptions options)
    {
    }

    // The storage returned from this factory is the default when the
    // application does not supply its own JobStorage instance. Its lifetime
    // is tied to the Hangfire global configuration (which sets
    // JobStorage.Current) and to the DI service provider that resolves the
    // IBackgroundProcessingServer hosted service: when the host is
    // disposed, the service provider disposes the registered storage, and
    // the InMemoryStorage's background Dispatcher joins its worker thread
    // before the storage becomes unreachable. Tests that want to share or
    // observe this lifetime should register a storage explicitly via the
    // AddPlatformHangfireJobs(Action<HangfireJobsOptions>?, JobStorage)
    // overload.
    private static JobStorage CreateStorage(HangfireJobsOptions options) => options.Storage switch
    {
        HangfireStorageKind.InMemory => new InMemoryStorage(),
        HangfireStorageKind.PostgreSql => new PostgreSqlStorage(
            new NpgsqlConnectionFactory(options.PostgreSqlConnectionString!, new PostgreSqlStorageOptions(), null),
            new PostgreSqlStorageOptions()),
        _ => throw new ArgumentOutOfRangeException(nameof(options), options.Storage, "The Hangfire storage kind is not supported."),
    };

    private sealed class HangfireJobsRegistrationMarker
    {
    }
}
