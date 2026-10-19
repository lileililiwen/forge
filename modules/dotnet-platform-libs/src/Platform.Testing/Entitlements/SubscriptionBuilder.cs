using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;
using Platform.Core.Time;

namespace Platform.Testing.Entitlements;

/// <summary>
/// Fluent builder for <see cref="Subscription"/> snapshots used in
/// tests. Defaults to an active subscription on the supplied
/// <see cref="PlanId"/> with a 30-day period starting at the supplied
/// clock's <c>UtcNow</c>.
/// </summary>
public sealed class SubscriptionBuilder
{
    private readonly SubjectKey _subject;
    private PlanId _plan = PlanId.Create("plan.test");
    private SubscriptionStatus _status = SubscriptionStatus.Active;
    private ProviderName _provider = ProviderName.Create("test");
    private string _providerSubscriptionId = "sub_test";
    private DateTimeOffset? _periodStart;
    private DateTimeOffset? _periodEnd;

    private SubscriptionBuilder(SubjectKey subject)
    {
        _subject = subject;
    }

    /// <summary>
    /// Starts a new builder for the supplied <paramref name="subject"/>.
    /// </summary>
    /// <param name="subject">The subject the subscription belongs to.</param>
    public static SubscriptionBuilder For(SubjectKey subject) => new(subject);

    /// <summary>Sets the plan identifier.</summary>
    public SubscriptionBuilder WithPlan(PlanId plan) { _plan = plan; return this; }

    /// <summary>Sets the normalized status.</summary>
    public SubscriptionBuilder WithStatus(SubscriptionStatus status) { _status = status; return this; }

    /// <summary>Sets the provider name.</summary>
    public SubscriptionBuilder WithProvider(ProviderName provider) { _provider = provider; return this; }

    /// <summary>Sets the opaque provider subscription identifier.</summary>
    public SubscriptionBuilder WithProviderSubscriptionId(string id) { _providerSubscriptionId = id; return this; }

    /// <summary>Sets the inclusive start of the current period.</summary>
    public SubscriptionBuilder WithPeriodStart(DateTimeOffset value) { _periodStart = value; return this; }

    /// <summary>Sets the exclusive end of the current period.</summary>
    public SubscriptionBuilder WithPeriodEnd(DateTimeOffset value) { _periodEnd = value; return this; }

    /// <summary>
    /// Builds the <see cref="Subscription"/>. When the period is not
    /// set, it is anchored at the supplied <paramref name="clock"/>
    /// (start = clock.UtcNow, end = start + 30 days).
    /// </summary>
    /// <param name="clock">The clock used to anchor the period.</param>
    public Subscription Build(IClock clock)
    {
        ArgumentNullException.ThrowIfNull(clock);
        var start = _periodStart ?? clock.UtcNow;
        var end = _periodEnd ?? start.AddDays(30);
        return new Subscription(_subject, _plan, _status, _provider, _providerSubscriptionId, start, end);
    }
}
