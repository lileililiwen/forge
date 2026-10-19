namespace Platform.Starter;

/// <summary>Explicit capability switches for a platform application.</summary>
public sealed class PlatformApplicationOptions
{
    /// <summary>Configuration section name used by starter hosts.</summary>
    public const string SectionName = "PlatformApplication";
    /// <summary>Enables the shared web runtime.</summary>
    public bool EnableWeb { get; set; } = true;
    /// <summary>Enables platform identity and authorization helpers.</summary>
    public bool EnableIdentity { get; set; }
    /// <summary>Enables the bounded administration surface.</summary>
    public bool EnableAdmin { get; set; }
    /// <summary>Enables the consumer-provided billing capability.</summary>
    public bool EnableBilling { get; set; }
    /// <summary>Enables a consumer-provided AI capability.</summary>
    public bool EnableAi { get; set; }
    /// <summary>Enables a consumer-provided notification capability.</summary>
    public bool EnableNotifications { get; set; }
    /// <summary>Enables a consumer-provided SMS capability.</summary>
    public bool EnableSms { get; set; }
    /// <summary>Environment name used for production configuration validation.</summary>
    public string EnvironmentName { get; set; } = "Development";
    /// <summary>Application-owned billing provider name.</summary>
    public string? BillingProviderName { get; set; }
    /// <summary>Application-owned AI provider name.</summary>
    public string? AiProviderName { get; set; }
    /// <summary>Application-owned notification provider name.</summary>
    public string? NotificationProviderName { get; set; }
    /// <summary>Application-owned SMS provider name.</summary>
    public string? SmsProviderName { get; set; }

    /// <summary>Returns safe configuration failures.</summary>
    public IReadOnlyList<string> Validate()
    {
        var failures = new List<string>();
        if (string.IsNullOrWhiteSpace(EnvironmentName)) failures.Add("EnvironmentName is required.");
        if (EnableAdmin && !EnableIdentity) failures.Add("EnableAdmin requires EnableIdentity.");
        if (IsProduction && EnableBilling && string.IsNullOrWhiteSpace(BillingProviderName)) failures.Add("BillingProviderName is required when billing is enabled in production.");
        if (IsProduction && EnableAi && string.IsNullOrWhiteSpace(AiProviderName)) failures.Add("AiProviderName is required when AI is enabled in production.");
        if (IsProduction && EnableNotifications && string.IsNullOrWhiteSpace(NotificationProviderName)) failures.Add("NotificationProviderName is required when notifications are enabled in production.");
        if (IsProduction && EnableSms && string.IsNullOrWhiteSpace(SmsProviderName)) failures.Add("SmsProviderName is required when SMS is enabled in production.");
        return failures;
    }

    /// <summary>Gets whether production-only provider checks apply.</summary>
    public bool IsProduction => string.Equals(EnvironmentName, "Production", StringComparison.OrdinalIgnoreCase);
}
