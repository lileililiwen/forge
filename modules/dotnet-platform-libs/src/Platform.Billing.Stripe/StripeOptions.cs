using Platform.Billing.Contracts.Plans;

namespace Platform.Billing.Stripe;

/// <summary>Configuration for the optional Stripe adapter.</summary>
public sealed class StripeOptions
{
    /// <summary>Stripe secret API key. It is never included in diagnostics.</summary>
    public string? ApiKey { get; set; }
    /// <summary>Stripe webhook signing secret.</summary>
    public string? WebhookSecret { get; set; }
    /// <summary>Stripe API base address.</summary>
    public Uri ApiBaseAddress { get; set; } = new("https://api.stripe.com/");
    /// <summary>HTTP timeout.</summary>
    public TimeSpan Timeout { get; set; } = TimeSpan.FromSeconds(10);
    /// <summary>Maximum retries for safe GET operations.</summary>
    public int MaxRetries { get; set; } = 2;
    /// <summary>Accepted webhook clock skew.</summary>
    public TimeSpan WebhookTolerance { get; set; } = TimeSpan.FromMinutes(5);
    /// <summary>Application-owned plan and provider reference catalog.</summary>
    public PlanCatalog PlanCatalog { get; set; } = new();
}
