using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Entitlements;

/// <summary>Consumer-owned persistence boundary for entitlement snapshots.</summary>
public interface IEntitlementStore
{
    /// <summary>Gets the current snapshot for a subject.</summary>
    ValueTask<Entitlement?> GetAsync(SubjectKey subject, CancellationToken cancellationToken = default);
    /// <summary>Replaces a subject snapshot.</summary>
    ValueTask SaveAsync(Entitlement entitlement, CancellationToken cancellationToken = default);
}
