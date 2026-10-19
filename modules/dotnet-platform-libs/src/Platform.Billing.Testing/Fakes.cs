#pragma warning disable CS1591
using Platform.Billing.Contracts;
using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Events;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Providers;
using Platform.Billing.Contracts.Subscriptions;
using Platform.Billing.Contracts.Usage;
using Platform.Core.Time;

namespace Platform.Billing.Testing;

/// <summary>In-memory entitlement snapshot store.</summary>
public sealed class InMemoryEntitlementStore : IEntitlementStore
{
    private readonly Dictionary<SubjectKey, Entitlement> _values = [];
    public ValueTask<Entitlement?> GetAsync(SubjectKey subject, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(_values.GetValueOrDefault(subject));
    public ValueTask SaveAsync(Entitlement entitlement, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(entitlement);
        _values[entitlement.Subject] = entitlement;
        return ValueTask.CompletedTask;
    }
}

/// <summary>Thread-safe in-memory usage meter with configurable limits.</summary>
public sealed class InMemoryUsageMeter : IUsageMeter
{
    private readonly Dictionary<(SubjectKey, FeatureKey), long> _usage = [];
    private readonly Dictionary<FeatureKey, long> _limits = [];
    private readonly object _gate = new();
    public void SetLimit(FeatureKey feature, long limit) { ArgumentOutOfRangeException.ThrowIfNegative(limit); lock (_gate) _limits[feature] = limit; }
    public Task<UsageCheckResult> CheckAsync(SubjectKey subject, FeatureKey feature, CancellationToken cancellationToken = default)
    {
        lock (_gate) return Task.FromResult(Result(subject, feature));
    }
    public Task<UsageCheckResult> RecordAsync(SubjectKey subject, FeatureKey feature, long units, CancellationToken cancellationToken = default)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(units);
        lock (_gate) { _usage[(subject, feature)] = _usage.GetValueOrDefault((subject, feature)) + units; return Task.FromResult(Result(subject, feature)); }
    }
    private UsageCheckResult Result(SubjectKey subject, FeatureKey feature) => new(feature, _usage.GetValueOrDefault((subject, feature)), _limits.GetValueOrDefault(feature), null, null);
}

/// <summary>Configurable provider fake with deterministic webhook normalization.</summary>
public sealed class InMemoryBillingProvider : IBillingProvider
{
    private readonly IClock _clock;
    private readonly Dictionary<SubjectKey, Subscription> _subscriptions = [];
    private readonly Dictionary<string, ProviderEvent> _webhooks = new(StringComparer.Ordinal);
    /// <summary>Initializes the fake for a provider name and clock.</summary>
    public InMemoryBillingProvider(ProviderName name, IClock clock) { Name = name; _clock = clock ?? throw new ArgumentNullException(nameof(clock)); }
    public ProviderName Name { get; }
    public void SetSubscription(Subscription subscription) => _subscriptions[subscription.Subject] = subscription;
    public void RegisterWebhook(string payload, ProviderEvent providerEvent) => _webhooks[payload] = providerEvent;
    public ValueTask<CheckoutSession> CreateCheckoutAsync(CheckoutRequest request, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(new CheckoutSession(Name, "checkout_" + request.Customer.Subject.Value, new Uri("https://billing.invalid/checkout/" + request.Customer.Subject.Value), _clock.UtcNow.AddMinutes(30)));
    public ValueTask<PortalSession> CreatePortalAsync(BillingCustomer customer, Uri returnUrl, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(new PortalSession(Name, "portal_" + customer.Subject.Value, returnUrl, _clock.UtcNow.AddMinutes(30)));
    public ValueTask<Subscription?> GetSubscriptionAsync(SubscriptionLookup lookup, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(_subscriptions.GetValueOrDefault(lookup.Subject));
    public ValueTask<Subscription?> CancelSubscriptionAsync(CancellationRequest request, CancellationToken cancellationToken = default)
    {
        if (!_subscriptions.TryGetValue(request.Subject, out var subscription)) return ValueTask.FromResult<Subscription?>(null);
        var canceled = subscription with { Status = SubscriptionStatus.Canceled };
        _subscriptions[request.Subject] = canceled;
        return ValueTask.FromResult<Subscription?>(canceled);
    }
    public ValueTask<WebhookNormalizationResult> VerifyAndNormalizeWebhookAsync(string payload, IReadOnlyDictionary<string, string> headers, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(_webhooks.TryGetValue(payload, out var value) ? new WebhookNormalizationResult(value) : new WebhookNormalizationResult(null, "invalid_webhook"));
    public ValueTask<BillingProviderStatus> GetStatusAsync(CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(new BillingProviderStatus(Name, true));
}
