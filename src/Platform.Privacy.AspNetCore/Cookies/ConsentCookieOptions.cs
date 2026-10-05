namespace Platform.Privacy.AspNetCore.Cookies;

/// <summary>Application-configured consent cookie shape.</summary>
public sealed class ConsentCookieOptions
{
    /// <summary>Cookie name carrying the consent record.</summary>
    public string CookieName { get; set; } = "platform_consent";

    /// <summary>How long a recorded decision lives.</summary>
    public TimeSpan Lifetime { get; set; } = TimeSpan.FromDays(365);

    /// <summary>Current policy version; bumping it voids older records' optional grants.</summary>
    public int PolicyVersion { get; set; } = 1;

    /// <summary>Validates the shape; throws on misuse.</summary>
    public void Validate()
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(CookieName);
        if (Lifetime <= TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(Lifetime), "Lifetime must be positive.");
        }

        if (PolicyVersion < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(PolicyVersion), "PolicyVersion must be at least 1.");
        }
    }
}
