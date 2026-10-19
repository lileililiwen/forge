namespace Platform.Jobs;

/// <summary>
/// Value type describing a single recurring job. The descriptor is the
/// unit the <see cref="IRecurringJobRegistry"/> stores and that
/// schedulers consume to attach a handler to a cron expression.
/// </summary>
/// <param name="Name">The stable, scheduler-visible name. MUST be non-null and non-empty.</param>
/// <param name="Cron">The documented cron expression. MUST be non-null and non-empty.</param>
/// <param name="HandlerType">The handler type implementing <see cref="IRecurringJobHandler"/>. MUST be non-null.</param>
/// <param name="TimeZone">The time zone identifier the scheduler applies to <paramref name="Cron"/>. MUST be non-null and non-empty.</param>
/// <param name="Options">Optional scheduler-specific options; <c>null</c> when no options are configured.</param>
public sealed record RecurringJobDescriptor(
    string Name,
    string Cron,
    Type HandlerType,
    string TimeZone = "UTC",
    IReadOnlyDictionary<string, object?>? Options = null)
{
    /// <summary>
    /// Returns a new descriptor with the supplied <paramref name="name"/>
    /// overriding <see cref="Name"/>. All other properties are copied
    /// verbatim.
    /// </summary>
    /// <param name="name">The new name.</param>
    public RecurringJobDescriptor WithName(string name) => this with { Name = name };

    /// <summary>
    /// Returns a new descriptor with the supplied <paramref name="cron"/>
    /// overriding <see cref="Cron"/>. All other properties are copied
    /// verbatim.
    /// </summary>
    /// <param name="cron">The new cron expression.</param>
    public RecurringJobDescriptor WithCron(string cron) => this with { Cron = cron };

    /// <summary>
    /// Returns a new descriptor with the supplied <paramref name="options"/>
    /// overriding <see cref="Options"/>. All other properties are copied
    /// verbatim.
    /// </summary>
    /// <param name="options">The new options bag.</param>
    public RecurringJobDescriptor WithOptions(IReadOnlyDictionary<string, object?>? options) =>
        this with { Options = options };
}
