namespace Platform.Adoption;

/// <summary>
/// Options controlling an adoption analysis run. All paths resolve relative
/// to the explicit target directory; nothing is inferred from the current
/// working directory.
/// </summary>
public sealed record AdoptionOptions
{
    /// <summary>
    /// Gets the expected exact <c>Platform.*</c> package version used to flag
    /// outdated pins and to build preview suggestions. Defaults to the
    /// current platform release line.
    /// </summary>
    public string ExpectedPlatformVersion { get; init; } = "0.1.0";

    /// <summary>
    /// Gets whether environment-dependent checks (feed reachability, Docker)
    /// run. When false, those checks are omitted entirely rather than
    /// reported as blocked.
    /// </summary>
    public bool IncludeEnvironmentChecks { get; init; }

    /// <summary>
    /// Gets an optional package feed URL probed by the environment feed
    /// check. Null disables the feed check.
    /// </summary>
    public string? FeedUrl { get; init; }

    /// <summary>
    /// Gets whether the Docker availability check runs as part of the
    /// environment checks. Defaults to false.
    /// </summary>
    public bool CheckDocker { get; init; }

    /// <summary>Gets the per-request timeout for environment probes.</summary>
    public TimeSpan EnvironmentProbeTimeout { get; init; } = TimeSpan.FromSeconds(5);
}
