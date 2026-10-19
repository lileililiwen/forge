namespace Platform.Notifications;

/// <summary>Notification runtime configuration.</summary>
public sealed class NotificationOptions
{
    /// <summary>Configuration section name.</summary>
    public const string SectionName = "Notifications";
    /// <summary>Environment name controlling console/fake safety.</summary>
    public string EnvironmentName { get; set; } = "Development";
    /// <summary>Maximum attempts for transient failures.</summary>
    public int MaxAttempts { get; set; } = 3;
    /// <summary>Configured provider label for diagnostics.</summary>
    public string? ProviderName { get; set; }
    /// <summary>Returns safe option failures.</summary>
    public IReadOnlyList<string> Validate() => new List<string>(MaxAttempts is >= 1 and <= 10 ? Array.Empty<string>() : ["MaxAttempts must be between 1 and 10."]);
    /// <summary>Gets whether this is a production environment.</summary>
    public bool IsProduction => string.Equals(EnvironmentName, "Production", StringComparison.OrdinalIgnoreCase);
}
