using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Identifiers;

namespace Platform.Testing.Entitlements;

/// <summary>
/// In-memory entitlement store for tests. Configures snapshots per
/// subject, supports invalidation, and exposes its state for
/// inspection. Adapters in the consuming application own the
/// mapping from a configured snapshot to the real subscription
/// state; the fake simply stores and returns what the test set up.
/// </summary>
public sealed class FakeEntitlementStore
{
    private readonly Dictionary<SubjectKey, Entitlement> _configured = new();
    private readonly HashSet<SubjectKey> _invalidated = new();

    /// <summary>
    /// Replaces the configured entitlement for the supplied
    /// <paramref name="subject"/> and clears any prior invalidation.
    /// </summary>
    /// <param name="subject">The subject the entitlement belongs to.</param>
    /// <param name="entitlement">The snapshot the next lookup returns.</param>
    public void Configure(SubjectKey subject, Entitlement entitlement)
    {
        ArgumentNullException.ThrowIfNull(entitlement);
        _configured[subject] = entitlement;
        _invalidated.Remove(subject);
    }

    /// <summary>
    /// Marks the supplied <paramref name="subject"/> as invalidated.
    /// Subsequent <see cref="FakeEntitlementStore.Get(SubjectKey, DateTimeOffset)"/>
    /// calls return the inactive default until
    /// <see cref="Configure(SubjectKey, Entitlement)"/> is called
    /// again.
    /// </summary>
    /// <param name="subject">The subject to invalidate.</param>
    public void Invalidate(SubjectKey subject)
    {
        _invalidated.Add(subject);
    }

    /// <summary>
    /// Returns the configured snapshot for the supplied
    /// <paramref name="subject"/>, or the inactive default when the
    /// subject is unknown or has been invalidated.
    /// </summary>
    /// <param name="subject">The subject to look up.</param>
    /// <param name="capturedAt">The UTC time stamped on the returned default.</param>
    public Entitlement Get(SubjectKey subject, DateTimeOffset capturedAt)
    {
        if (_invalidated.Contains(subject))
        {
            return EntitlementDefaults.Inactive(subject, capturedAt);
        }
        return _configured.TryGetValue(subject, out var snapshot)
            ? snapshot
            : EntitlementDefaults.Inactive(subject, capturedAt);
    }

    /// <summary>
    /// Gets a read-only view of the subjects that have been
    /// invalidated since the last <see cref="Configure(SubjectKey, Entitlement)"/>
    /// or invalidation reset.
    /// </summary>
    public IReadOnlyCollection<SubjectKey> InvalidatedSubjects => _invalidated.ToArray();

    /// <summary>
    /// Clears all configured entitlements and invalidations.
    /// </summary>
    public void Reset()
    {
        _configured.Clear();
        _invalidated.Clear();
    }
}
