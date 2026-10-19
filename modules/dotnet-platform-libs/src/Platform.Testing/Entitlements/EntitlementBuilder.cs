using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;
using Platform.Core.Time;

namespace Platform.Testing.Entitlements;

/// <summary>
/// Fluent builder for <see cref="Entitlement"/> snapshots used in
/// tests. Defaults to an active entitlement with no granted
/// features and no limits. Use <see cref="EntitlementBuilder.Granting(FeatureKey)"/>
/// to flip the snapshot into a state that grants a feature set.
/// </summary>
public sealed class EntitlementBuilder
{
    private readonly SubjectKey _subject;
    private SubjectKey? _tenant;
    private Subscription? _subscription;
    private readonly HashSet<FeatureKey> _features = new();
    private readonly Dictionary<FeatureKey, long> _limits = new();
    private DateTimeOffset? _capturedAt;

    private EntitlementBuilder(SubjectKey subject)
    {
        _subject = subject;
    }

    /// <summary>Starts a builder for the supplied <paramref name="subject"/>.</summary>
    public static EntitlementBuilder For(SubjectKey subject) => new(subject);

    /// <summary>Sets the optional tenant scope.</summary>
    public EntitlementBuilder WithTenant(SubjectKey tenant) { _tenant = tenant; return this; }

    /// <summary>Replaces the underlying subscription.</summary>
    public EntitlementBuilder WithSubscription(Subscription subscription)
    {
        _subscription = subscription;
        return this;
    }

    /// <summary>Adds a single granted feature.</summary>
    public EntitlementBuilder Granting(FeatureKey feature) { _features.Add(feature); return this; }

    /// <summary>Adds multiple granted features.</summary>
    public EntitlementBuilder Granting(IEnumerable<FeatureKey> features)
    {
        ArgumentNullException.ThrowIfNull(features);
        foreach (var feature in features)
        {
            _features.Add(feature);
        }
        return this;
    }

    /// <summary>Adds a single numeric limit for the supplied feature.</summary>
    public EntitlementBuilder WithLimit(FeatureKey feature, long limit)
    {
        _limits[feature] = limit;
        return this;
    }

    /// <summary>Sets the captured-at UTC time. Defaults to the clock passed to <see cref="Build(IClock)"/>.</summary>
    public EntitlementBuilder CapturedAt(DateTimeOffset value) { _capturedAt = value; return this; }

    /// <summary>Builds the <see cref="Entitlement"/> snapshot.</summary>
    public Entitlement Build(IClock clock)
    {
        ArgumentNullException.ThrowIfNull(clock);
        var captured = _capturedAt ?? clock.UtcNow;
        var limits = _limits.Count == 0 ? null : (IReadOnlyDictionary<FeatureKey, long>?)new Dictionary<FeatureKey, long>(_limits);
        return new Entitlement(
            _subject,
            _tenant,
            _subscription,
            _features.ToHashSet(),
            limits,
            captured);
    }
}
