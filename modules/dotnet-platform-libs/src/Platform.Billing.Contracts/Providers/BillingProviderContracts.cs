using Platform.Billing.Contracts.Events;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;

namespace Platform.Billing.Contracts.Providers;

/// <summary>Normalized customer identity sent to a billing provider.</summary>
public sealed record BillingCustomer(SubjectKey Subject, string? Email = null, string? Name = null);
/// <summary>Provider-neutral checkout request.</summary>
public sealed record CheckoutRequest(BillingCustomer Customer, PlanId Plan, Uri SuccessUrl, Uri CancelUrl, string? Tenant = null);
/// <summary>Provider-neutral checkout session.</summary>
public sealed record CheckoutSession(ProviderName Provider, string SessionId, Uri CheckoutUrl, DateTimeOffset ExpiresAt);
/// <summary>Provider-neutral customer portal session.</summary>
public sealed record PortalSession(ProviderName Provider, string SessionId, Uri PortalUrl, DateTimeOffset ExpiresAt);
/// <summary>Provider-neutral subscription lookup.</summary>
public sealed record SubscriptionLookup(SubjectKey Subject, string? ProviderSubscriptionId = null);
/// <summary>Provider-neutral cancellation request.</summary>
public sealed record CancellationRequest(SubjectKey Subject, bool AtPeriodEnd = true, string? Reason = null);
/// <summary>Provider availability projection.</summary>
public sealed record BillingProviderStatus(ProviderName Provider, bool Available, string? Detail = null);
/// <summary>Result of webhook verification and normalization.</summary>
public sealed record WebhookNormalizationResult(ProviderEvent? Event, string? Error = null)
{
    /// <summary>Gets whether a normalized event was produced.</summary>
    public bool Succeeded => Event is not null && Error is null;
}

/// <summary>Provider boundary for checkout, portal, subscription and webhooks.</summary>
public interface IBillingProvider
{
    /// <summary>Gets the opaque provider name.</summary>
    ProviderName Name { get; }
    /// <summary>Creates a checkout session.</summary>
    ValueTask<CheckoutSession> CreateCheckoutAsync(CheckoutRequest request, CancellationToken cancellationToken = default);
    /// <summary>Creates a customer portal session.</summary>
    ValueTask<PortalSession> CreatePortalAsync(BillingCustomer customer, Uri returnUrl, CancellationToken cancellationToken = default);
    /// <summary>Looks up a normalized subscription.</summary>
    ValueTask<Subscription?> GetSubscriptionAsync(SubscriptionLookup lookup, CancellationToken cancellationToken = default);
    /// <summary>Requests cancellation using provider-neutral semantics.</summary>
    ValueTask<Subscription?> CancelSubscriptionAsync(CancellationRequest request, CancellationToken cancellationToken = default);
    /// <summary>Verifies and normalizes a raw webhook.</summary>
    ValueTask<WebhookNormalizationResult> VerifyAndNormalizeWebhookAsync(string payload, IReadOnlyDictionary<string, string> headers, CancellationToken cancellationToken = default);
    /// <summary>Gets provider health.</summary>
    ValueTask<BillingProviderStatus> GetStatusAsync(CancellationToken cancellationToken = default);
}
