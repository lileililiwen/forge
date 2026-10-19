using Hangfire;
using Hangfire.InMemory;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.Jobs.Hangfire.DependencyInjection;

namespace Platform.Jobs.Hangfire.Tests;

/// <summary>
/// Isolated end-to-end Hangfire test host. Each instance owns a fresh
/// <see cref="InMemoryStorage"/>, service provider, and hosted background
/// server so tests never share dispatcher state across runs. The host
/// waits for the background server to register itself with the storage
/// before enqueuing so jobs are never dropped on startup, and disposes
/// the host in an order that lets the in-memory dispatcher's worker
/// thread join before the service provider is torn down. The test host
/// is intentionally minimal: it does not own assertions, telemetry, or
/// platform handlers — those are wired by the test through the
/// <see cref="Builder.ConfigureServices"/> hook.
/// </summary>
public sealed class HangfireEndToEndHost : IAsyncDisposable
{
    private readonly IHost _host;
    private readonly InMemoryStorage _storage;
    private bool _disposed;

    private HangfireEndToEndHost(IHost host, InMemoryStorage storage)
    {
        _host = host;
        _storage = storage;
    }

    /// <summary>The built host; safe to use after <see cref="StartAsync(CancellationToken)"/>.</summary>
    public IHost Host => _host;

    /// <summary>The isolated in-memory storage owned by this host.</summary>
    public InMemoryStorage Storage => _storage;

    /// <summary>The platform service provider, exposed so tests can resolve platform services.</summary>
    public IServiceProvider Services => _host.Services;

    /// <summary>Starts the host and blocks until the background server has registered itself with the storage.</summary>
    /// <param name="cancellationToken">Cancels the readiness wait.</param>
    public Task StartAsync(CancellationToken cancellationToken = default) =>
        StartAsync(WorkerReadyTimeout, cancellationToken);

    /// <summary>Starts the host and blocks until the background server has registered itself with the storage or the timeout elapses.</summary>
    /// <param name="readyTimeout">Maximum time to wait for the worker to register.</param>
    /// <param name="cancellationToken">Cancels the readiness wait.</param>
    public async Task StartAsync(TimeSpan readyTimeout, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(_host);
        await _host.StartAsync(cancellationToken);
        await WaitForWorkerReadyAsync(readyTimeout, cancellationToken);
    }

    /// <summary>
    /// Polls the storage until the background server has registered itself
    /// or the configured timeout elapses. Fails fast instead of hanging the
    /// test on a misconfigured host.
    /// </summary>
    public async Task WaitForWorkerReadyAsync(TimeSpan timeout, CancellationToken cancellationToken = default)
    {
        if (timeout <= TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(timeout), timeout, "The worker-ready timeout must be positive.");
        }

        var deadline = DateTimeOffset.UtcNow + timeout;
        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();

            try
            {
                var servers = _storage.GetMonitoringApi().Servers();
                if (servers.Count > 0)
                {
                    return;
                }
            }
            catch (ObjectDisposedException)
            {
                throw new InvalidOperationException("The Hangfire storage was disposed before the worker could register.");
            }

            if (DateTimeOffset.UtcNow >= deadline)
            {
                throw new TimeoutException(
                    $"The Hangfire background server did not register itself within {timeout.TotalSeconds:0.##} seconds.");
            }

            try
            {
                await Task.Delay(WorkerReadyPollInterval, cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
        }
    }

    /// <summary>Returns true after the host has been disposed.</summary>
    public bool IsDisposed => _disposed;

    /// <summary>
    /// Disposes the host. The background server is stopped first
    /// (waiting for in-flight jobs to complete), then the service
    /// provider is disposed, which disposes the owned storage and
    /// joins the in-memory dispatcher's worker thread. Safe to call
    /// multiple times.
    /// </summary>
    public async ValueTask DisposeAsync()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        try
        {
            using var stopCts = new CancellationTokenSource(ShutdownTimeout);
            await _host.StopAsync(stopCts.Token);
        }
        catch (OperationCanceledException)
        {
            // The host did not stop within the bounded shutdown timeout.
            // Continue to disposal so the test can still report a clean
            // storage-disposed state and not leak the dispatcher's thread.
        }

        if (_host is IAsyncDisposable asyncDisposable)
        {
            await asyncDisposable.DisposeAsync();
        }
        else
        {
            _host.Dispose();
        }
    }

    /// <summary>Maximum time the host is given to shut down on disposal.</summary>
    public TimeSpan ShutdownTimeout { get; set; } = TimeSpan.FromSeconds(15);

    private static readonly TimeSpan WorkerReadyTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan WorkerReadyPollInterval = TimeSpan.FromMilliseconds(50);

    /// <summary>Builds a fresh <see cref="HangfireEndToEndHost"/> with an isolated in-memory storage.</summary>
    public static Builder CreateBuilder() => new();

    /// <summary>Fluent builder for <see cref="HangfireEndToEndHost"/>.</summary>
    public sealed class Builder
    {
        private readonly List<Action<IServiceCollection>> _configure = new();
        private Action<HangfireJobsOptions>? _configureOptions;
        private InMemoryStorage? _storage;

        /// <summary>Registers an additional configuration delegate against the host's service collection.</summary>
        public Builder ConfigureServices(Action<IServiceCollection> configure)
        {
            ArgumentNullException.ThrowIfNull(configure);
            _configure.Add(configure);
            return this;
        }

        /// <summary>Configures the platform Hangfire options before the host is built.</summary>
        public Builder ConfigureOptions(Action<HangfireJobsOptions> configure)
        {
            ArgumentNullException.ThrowIfNull(configure);
            _configureOptions = configure;
            return this;
        }

        /// <summary>Supplies an explicit in-memory storage; otherwise a fresh one is created per host.</summary>
        public Builder UseStorage(InMemoryStorage storage)
        {
            ArgumentNullException.ThrowIfNull(storage);
            _storage = storage;
            return this;
        }

        /// <summary>Builds a new host. The returned host has not been started.</summary>
        public HangfireEndToEndHost Build()
        {
            // Force the global Hangfire log provider to a no-op before
            // building storage: the static log provider still references
            // the previous host's ILoggerFactory, which is disposed when
            // that host shuts down, and the next host's in-memory
            // dispatcher would observe a disposed logger factory on
            // construction. The new AddHangfire callback will rebind the
            // log provider to the new host's logger factory when the
            // IGlobalConfiguration factory runs.
            GlobalConfiguration.Configuration.UseNoOpLogProvider();
            var storage = _storage ?? new InMemoryStorage();
            var hostBuilder = Microsoft.Extensions.Hosting.Host.CreateApplicationBuilder();
            hostBuilder.Services.AddLogging();
            hostBuilder.Services.AddPlatformHangfireJobs(_configureOptions, storage);
            foreach (var configure in _configure)
            {
                configure(hostBuilder.Services);
            }
            return new HangfireEndToEndHost(hostBuilder.Build(), storage);
        }
    }
}
