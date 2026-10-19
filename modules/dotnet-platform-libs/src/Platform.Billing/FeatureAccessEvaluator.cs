using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Features;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Usage;
using Platform.Core.Time;

namespace Platform.Billing;

/// <summary>Combines entitlement state and usage state into one explainable decision.</summary>
public sealed class FeatureAccessEvaluator
{
    private readonly IUsageMeter _usage;
    private readonly IClock _clock;
    /// <summary>Initializes the evaluator.</summary>
    public FeatureAccessEvaluator(IUsageMeter usage, IClock clock)
    {
        ArgumentNullException.ThrowIfNull(usage);
        ArgumentNullException.ThrowIfNull(clock);
        _usage = usage; _clock = clock;
    }
    /// <summary>Checks a feature and includes usage details when available.</summary>
    public async ValueTask<FeatureCheckResult> CheckAsync(Entitlement entitlement, FeatureKey feature, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(entitlement);
        var decision = FeatureCheck.Evaluate(entitlement, feature, _clock.UtcNow);
        if (!decision.IsAllowed) return decision;
        var usage = await _usage.CheckAsync(entitlement.Subject, feature, cancellationToken);
        return usage.IsWithinLimit
            ? decision with { CurrentUsage = usage.Used, Limit = usage.Limit }
            : decision with { Reason = FeatureCheckReason.LimitExceeded, CurrentUsage = usage.Used, Limit = usage.Limit };
    }
}
