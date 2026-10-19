using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Plans;

/// <summary>Application-owned normalized plan and feature definition.</summary>
public sealed record BillingPlan(PlanId Id, string DisplayName, IReadOnlyDictionary<FeatureKey, long>? FeatureLimits = null)
{
    /// <summary>Gets normalized limits, or an empty dictionary.</summary>
    public IReadOnlyDictionary<FeatureKey, long> Limits => FeatureLimits ?? new Dictionary<FeatureKey, long>();
}

/// <summary>Opaque provider-specific reference owned by the application.</summary>
public sealed record ProviderPlanReference(ProviderName Provider, string ProviderValue);

/// <summary>Provider-neutral plan catalog.</summary>
public sealed class PlanCatalog
{
    private readonly Dictionary<PlanId, BillingPlan> _plans = [];
    private readonly Dictionary<(PlanId, ProviderName), ProviderPlanReference> _references = [];

    /// <summary>Adds a plan and its provider mapping.</summary>
    public PlanCatalog Add(BillingPlan plan, ProviderPlanReference reference)
    {
        ArgumentNullException.ThrowIfNull(plan);
        ArgumentNullException.ThrowIfNull(reference);
        if (string.IsNullOrWhiteSpace(reference.ProviderValue)) throw new ArgumentException("Provider reference is required.", nameof(reference));
        if (_plans.TryGetValue(plan.Id, out var existing) && existing != plan) throw new InvalidOperationException("Plan is already registered with different details: " + plan.Id.Value);
        _plans[plan.Id] = plan;
        if (!_references.TryAdd((plan.Id, reference.Provider), reference)) throw new InvalidOperationException("Provider plan reference is already registered.");
        return this;
    }

    /// <summary>Adds a plan and all of its provider mappings.</summary>
    public PlanCatalog Add(BillingPlan plan, IEnumerable<ProviderPlanReference> references)
    {
        ArgumentNullException.ThrowIfNull(references);
        foreach (var reference in references) Add(plan, reference);
        return this;
    }

    /// <summary>Gets the provider mapping for a plan.</summary>
    public ProviderPlanReference ReferenceFor(PlanId plan, ProviderName provider) =>
        _references.TryGetValue((plan, provider), out var reference)
            ? reference
            : throw new KeyNotFoundException("No provider reference is registered for the plan.");

    /// <summary>Gets the feature limits for a plan.</summary>
    public IReadOnlyDictionary<FeatureKey, long> FeaturesFor(PlanId plan) =>
        _plans.TryGetValue(plan, out var value) ? value.Limits : throw new KeyNotFoundException("Plan is not registered: " + plan.Value);
}
