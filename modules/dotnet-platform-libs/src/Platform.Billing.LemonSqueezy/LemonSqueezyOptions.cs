using Platform.Billing.Contracts.Plans;

namespace Platform.Billing.LemonSqueezy;

/// <summary>Configuration for the optional Lemon Squeezy adapter.</summary>
public sealed class LemonSqueezyOptions
{
    /// <summary>Lemon Squeezy API key.</summary>
    public string? ApiKey { get; set; }
    /// <summary>Lemon Squeezy signing secret.</summary>
    public string? WebhookSecret { get; set; }
    /// <summary>Lemon Squeezy API base address.</summary>
    public Uri ApiBaseAddress { get; set; } = new("https://api.lemonsqueezy.com/v1/");
    /// <summary>HTTP timeout.</summary>
    public TimeSpan Timeout { get; set; } = TimeSpan.FromSeconds(10);
    /// <summary>Maximum retries for safe GET operations.</summary>
    public int MaxRetries { get; set; } = 2;
    /// <summary>Application-owned plan and provider reference catalog.</summary>
    public PlanCatalog PlanCatalog { get; set; } = new();
}
