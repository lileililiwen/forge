namespace Platform.Adoption;

/// <summary>
/// Classification of a single adoption diagnostic result.
/// </summary>
public enum AdoptionStatus
{
    /// <summary>The check passed; no action required.</summary>
    Pass,

    /// <summary>The check found a non-blocking concern with remediation guidance.</summary>
    Warning,

    /// <summary>The check found a source problem that should fail CI adoption gates.</summary>
    Failed,

    /// <summary>
    /// The check could not run because an external dependency (feed, Docker,
    /// service) was unavailable. Never a claim about consumer source validity.
    /// </summary>
    EnvironmentBlocked,
}
