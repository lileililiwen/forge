namespace Platform.Jobs;

/// <summary>
/// Configuration bound to the <c>BackgroundJobs</c> configuration
/// section by the
/// <c>Platform.Jobs.DependencyInjection.ServiceCollectionExtensions.AddPlatformJobs</c>
/// extension. All properties have safe defaults; consumers override
/// only what they need.
/// </summary>
public sealed class BackgroundJobsOptions
{
    /// <summary>
    /// The configuration section name bound by
    /// <c>Platform.Jobs.DependencyInjection.ServiceCollectionExtensions.AddPlatformJobs</c>.
    /// </summary>
    public const string SectionName = "BackgroundJobs";

    /// <summary>
    /// Gets or sets the default time zone identifier the dispatcher
    /// applies to cron expressions that do not declare their own
    /// time zone. Defaults to <c>UTC</c>.
    /// </summary>
    public string DefaultTimeZone { get; set; } = "UTC";
}
