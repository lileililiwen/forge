namespace Platform.Mailing;

/// <summary>
/// Configuration bound to the <c>Mailing</c> configuration section by
/// the
/// <c>Platform.Mailing.DependencyInjection.ServiceCollectionExtensions.AddPlatformMailing</c>
/// extension. All properties have safe defaults; consumers override
/// only what they need.
/// </summary>
public sealed class MailingOptions
{
    /// <summary>
    /// The configuration section name bound by
    /// <c>Platform.Mailing.DependencyInjection.ServiceCollectionExtensions.AddPlatformMailing</c>.
    /// </summary>
    public const string SectionName = "Mailing";

    /// <summary>
    /// Gets or sets the default sender address. Defaults to
    /// <c>noreply@example.invalid</c>.
    /// </summary>
    public string DefaultFromAddress { get; set; } = "noreply@example.invalid";

    /// <summary>
    /// Gets or sets the default sender display name. Defaults to
    /// <c>Platform</c>.
    /// </summary>
    public string DefaultFromDisplayName { get; set; } = "Platform";

    /// <summary>
    /// Gets or sets the maximum number of send attempts on transient
    /// failures. Defaults to <c>3</c>.
    /// </summary>
    public int MaxAttempts { get; set; } = 3;

    /// <summary>
    /// Gets or sets the initial back-off delay, in seconds, applied
    /// between retries. Defaults to <c>5</c>.
    /// </summary>
    public int InitialBackoffSeconds { get; set; } = 5;

    /// <summary>
    /// Gets or sets the maximum back-off delay, in seconds, applied
    /// between retries. Defaults to <c>60</c>.
    /// </summary>
    public int MaxBackoffSeconds { get; set; } = 60;

    /// <summary>
    /// Gets or sets the path the renderer reads templates from. The
    /// value is renderer-specific; the platform does not interpret it.
    /// Defaults to <c>templates</c>.
    /// </summary>
    public string TemplatesPath { get; set; } = "templates";
}
