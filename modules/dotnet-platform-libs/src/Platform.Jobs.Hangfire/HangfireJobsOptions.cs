using Hangfire.Dashboard;

namespace Platform.Jobs.Hangfire;

/// <summary>
/// Configuration for the Hangfire adapter. All values are bounded and
/// validated by <see cref="Validate"/> at registration; error messages
/// never echo secrets or connection strings.
/// </summary>
public sealed class HangfireJobsOptions
{
    /// <summary>
    /// The configuration section name applications typically bind this
    /// options type from.
    /// </summary>
    public const string SectionName = "BackgroundJobs:Hangfire";

    /// <summary>
    /// Gets or sets the storage backend. Defaults to
    /// <see cref="HangfireStorageKind.InMemory"/>; production hosts
    /// typically select <see cref="HangfireStorageKind.PostgreSql"/>.
    /// </summary>
    public HangfireStorageKind Storage { get; set; } = HangfireStorageKind.InMemory;

    /// <summary>
    /// Gets or sets the PostgreSQL connection string used when
    /// <see cref="Storage"/> is <see cref="HangfireStorageKind.PostgreSql"/>.
    /// The value is supplied by the application and is never written to
    /// diagnostics.
    /// </summary>
    public string? PostgreSqlConnectionString { get; set; }

    /// <summary>
    /// Gets or sets the queue dispatched payloads are enqueued to.
    /// Defaults to <c>default</c>.
    /// </summary>
    public string Queue { get; set; } = "default";

    /// <summary>
    /// Gets or sets the queues the Hangfire server listens on.
    /// Defaults to a single <c>default</c> queue.
    /// </summary>
    public IReadOnlyList<string> Queues { get; set; } = new[] { "default" };

    /// <summary>
    /// Gets or sets the Hangfire worker count. Defaults to <c>5</c>;
    /// must be between 1 and 100.
    /// </summary>
    public int WorkerCount { get; set; } = 5;

    /// <summary>
    /// Gets or sets how often the Hangfire server polls for scheduled and
    /// recurring work. Defaults to 30 seconds; must be between 1 second
    /// and 10 minutes.
    /// </summary>
    public TimeSpan SchedulePollingInterval { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>
    /// Gets or sets the Hangfire server heartbeat interval. Defaults to
    /// 30 seconds; must be between 1 second and 10 minutes.
    /// </summary>
    public TimeSpan HeartbeatInterval { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>
    /// Gets or sets a value indicating whether the Hangfire dashboard may
    /// be mapped. Defaults to <c>false</c>; the dashboard is never mapped
    /// unless the host opts in explicitly.
    /// </summary>
    public bool DashboardEnabled { get; set; }

    /// <summary>
    /// Gets or sets the request path the dashboard is mapped to when
    /// <see cref="DashboardEnabled"/> is <c>true</c>. Defaults to
    /// <c>/jobs</c>.
    /// </summary>
    public string DashboardRoute { get; set; } = "/jobs";

    /// <summary>
    /// Gets or sets the application-provided dashboard authorization
    /// callback. Required when <see cref="DashboardEnabled"/> is
    /// <c>true</c>; the platform never ships a default authorization
    /// policy or credentials.
    /// </summary>
    public Func<DashboardContext, bool>? DashboardAuthorization { get; set; }

    /// <summary>
    /// Validates the configuration. Secret values are never included in
    /// the exception messages.
    /// </summary>
    /// <exception cref="ArgumentException">A required string is missing or malformed.</exception>
    /// <exception cref="ArgumentOutOfRangeException">A numeric or time-span value is outside the documented bounds.</exception>
    public void Validate()
    {
        ValidateQueueName(Queue, nameof(Queue));

        if (Queues is null || Queues.Count == 0)
        {
            throw new ArgumentException("At least one Hangfire queue is required.", nameof(Queues));
        }

        if (Queues.Count > 20)
        {
            throw new ArgumentException("At most 20 Hangfire queues are supported.", nameof(Queues));
        }

        foreach (var queue in Queues)
        {
            ValidateQueueName(queue, nameof(Queues));
        }

        if (WorkerCount is < 1 or > 100)
        {
            throw new ArgumentOutOfRangeException(nameof(WorkerCount), WorkerCount, "The Hangfire worker count must be between 1 and 100.");
        }

        ValidateInterval(SchedulePollingInterval, nameof(SchedulePollingInterval));
        ValidateInterval(HeartbeatInterval, nameof(HeartbeatInterval));

        if (Storage == HangfireStorageKind.PostgreSql && string.IsNullOrWhiteSpace(PostgreSqlConnectionString))
        {
            throw new ArgumentException("A PostgreSQL connection string is required when the Hangfire storage kind is PostgreSql.", nameof(PostgreSqlConnectionString));
        }

        if (PostgreSqlConnectionString is { Length: > 4096 })
        {
            throw new ArgumentException("The PostgreSQL connection string exceeds the documented length limit.", nameof(PostgreSqlConnectionString));
        }

        if (string.IsNullOrWhiteSpace(DashboardRoute))
        {
            throw new ArgumentException("A dashboard route is required when the dashboard is configured.", nameof(DashboardRoute));
        }

        if (!DashboardRoute.StartsWith('/'))
        {
            throw new ArgumentException("The dashboard route must start with '/'.", nameof(DashboardRoute));
        }

        if (DashboardRoute.Length > 200 || DashboardRoute.Any(static c => char.IsWhiteSpace(c)))
        {
            throw new ArgumentException("The dashboard route must be at most 200 characters without whitespace.", nameof(DashboardRoute));
        }

        if (DashboardEnabled && DashboardAuthorization is null)
        {
            throw new ArgumentException("A dashboard authorization callback is required when the dashboard is enabled.", nameof(DashboardAuthorization));
        }
    }

    private static void ValidateQueueName(string? queue, string parameterName)
    {
        if (string.IsNullOrWhiteSpace(queue))
        {
            throw new ArgumentException("A Hangfire queue name must be a non-empty string.", parameterName);
        }

        if (queue.Length > 128 || queue.Any(static c => char.IsWhiteSpace(c)))
        {
            throw new ArgumentException("A Hangfire queue name must be at most 128 characters without whitespace.", parameterName);
        }
    }

    private static void ValidateInterval(TimeSpan interval, string parameterName)
    {
        if (interval < TimeSpan.FromSeconds(1) || interval > TimeSpan.FromMinutes(10))
        {
            throw new ArgumentOutOfRangeException(parameterName, interval, "The interval must be between 1 second and 10 minutes.");
        }
    }
}
