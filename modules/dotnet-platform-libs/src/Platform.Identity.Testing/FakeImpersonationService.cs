using System.Collections.Concurrent;
using System.Globalization;
using Platform.Identity.Contracts;

namespace Platform.Identity.Testing;

/// <summary>
/// Deterministic impersonation service. The fake delegates every authorization
/// decision to a swappable <see cref="IImpersonationPolicy"/> and tracks active
/// grants in memory. Defaults to <see cref="DenyAllImpersonationPolicy"/>.
/// </summary>
public sealed class FakeImpersonationService : IImpersonationService
{
    private readonly ConcurrentDictionary<string, ImpersonationGrant> _grants = new(StringComparer.Ordinal);
    private readonly IImpersonationPolicy _policy;
    private readonly IIdentityAuditHook? _audit;
    private long _sequence;

    /// <summary>Creates a new fake impersonation service.</summary>
    public FakeImpersonationService(IImpersonationPolicy? policy = null, IIdentityAuditHook? audit = null)
    {
        _policy = policy ?? new DenyAllImpersonationPolicy();
        _audit = audit;
    }

    /// <summary>Records every impersonation grant issued by the fake.</summary>
    public ConcurrentBag<ImpersonationGrant> IssuedGrants { get; } = new();

    /// <inheritdoc />
    public async ValueTask<IdentityLifecycleResult<ImpersonationGrant>> StartAsync(ImpersonationAuthorizationRequest request, CancellationToken cancellationToken = default)
    {
        if (!request.IsValid)
        {
            await RecordAsync("identity.impersonation.denied", request.CallerSubjectId, false, cancellationToken).ConfigureAwait(false);
            return IdentityLifecycleResults.Failed<ImpersonationGrant>(IdentityLifecycleOutcome.InvalidRequest);
        }
        var policyResult = await _policy.AuthorizeAsync(request, cancellationToken).ConfigureAwait(false);
        if (!policyResult.Succeeded)
        {
            await RecordAsync("identity.impersonation.denied", request.CallerSubjectId, false, cancellationToken).ConfigureAwait(false);
            return IdentityLifecycleResults.Failed<ImpersonationGrant>(policyResult.Outcome);
        }
        var grant = new ImpersonationGrant(
            GrantId: "imp_" + Interlocked.Increment(ref _sequence).ToString("D20", CultureInfo.InvariantCulture),
            CallerSubjectId: request.CallerSubjectId,
            TargetSubjectId: request.TargetSubjectId,
            StartedAt: DateTimeOffset.UtcNow,
            ExpiresAt: DateTimeOffset.UtcNow.Add(request.Duration),
            Reason: request.Reason!);
        _grants[grant.GrantId] = grant;
        IssuedGrants.Add(grant);
        await RecordAsync("identity.impersonation.started", request.CallerSubjectId, true, cancellationToken).ConfigureAwait(false);
        return IdentityLifecycleResults.Success(grant);
    }

    /// <inheritdoc />
    public async ValueTask<IdentityLifecycleOutcome> EndAsync(string grantId, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(grantId))
            return IdentityLifecycleOutcome.InvalidHandle;
        if (!_grants.TryGetValue(grantId, out var grant))
        {
            await RecordAsync("identity.impersonation.ended", null, false, cancellationToken).ConfigureAwait(false);
            return IdentityLifecycleOutcome.InvalidHandle;
        }
        _grants.TryRemove(grantId, out _);
        await RecordAsync("identity.impersonation.ended", grant.CallerSubjectId, true, cancellationToken).ConfigureAwait(false);
        return IdentityLifecycleOutcome.Succeeded;
    }

    /// <inheritdoc />
    public ValueTask<IdentityLifecycleResult<ImpersonationContext>> GetActiveAsync(string callerSubjectId, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(callerSubjectId))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<ImpersonationContext>(IdentityLifecycleOutcome.InvalidRequest));
        var now = DateTimeOffset.UtcNow;
        var grant = _grants.Values.FirstOrDefault(g => g.CallerSubjectId == callerSubjectId && !g.IsExpired(now));
        return ValueTask.FromResult(grant is null
            ? IdentityLifecycleResults.Success(ImpersonationContext.None)
            : IdentityLifecycleResults.Success(new ImpersonationContext(grant.GrantId, grant.CallerSubjectId, grant.TargetSubjectId)));
    }

    private ValueTask RecordAsync(string action, string? subjectId, bool succeeded, CancellationToken cancellationToken) =>
        _audit is null
            ? ValueTask.CompletedTask
            : _audit.RecordAsync(new IdentityAuditEvent(action, subjectId, succeeded, DateTimeOffset.UtcNow), cancellationToken);
}

/// <summary>Policy that denies every impersonation request. Used as the default.</summary>
public sealed class DenyAllImpersonationPolicy : IImpersonationPolicy
{
    /// <inheritdoc />
    public ValueTask<IdentityLifecycleResult<bool>> AuthorizeAsync(ImpersonationAuthorizationRequest request, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(IdentityLifecycleResults.Failed<bool>(IdentityLifecycleOutcome.PolicyDenied));
}

/// <summary>Policy that allows every well-formed impersonation request. Tests can extend or replace it.</summary>
public sealed class AllowImpersonationPolicy : IImpersonationPolicy
{
    /// <inheritdoc />
    public ValueTask<IdentityLifecycleResult<bool>> AuthorizeAsync(ImpersonationAuthorizationRequest request, CancellationToken cancellationToken = default) =>
        ValueTask.FromResult(IdentityLifecycleResults.Success(true));
}
