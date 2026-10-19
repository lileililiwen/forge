using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.Tenant.Lifecycle.AspNetCore;

/// <summary>Provider-neutral readiness integration for tenant lifecycle. Maps the operation state to the platform readiness taxonomy.</summary>
public static class TenantLifecycleReadinessServiceCollectionExtensions
{
    /// <summary>Registers the readiness check that reports the most recent operation for a tenant.</summary>
    public static IServiceCollection AddPlatformTenantLifecycleReadiness(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddEnumerable(ServiceDescriptor.Singleton<IReadinessCheck, TenantLifecycleReadinessCheck>());
        return services;
    }
}

/// <summary>Default readiness check that reports the most recent operation state for the resolved tenant.</summary>
public sealed class TenantLifecycleReadinessCheck : IReadinessCheck
{
    private readonly ITenantLifecycleStore _store;

    /// <summary>Creates a new readiness check.</summary>
    public TenantLifecycleReadinessCheck(ITenantLifecycleStore store)
    {
        _store = store ?? throw new ArgumentNullException(nameof(store));
    }

    /// <inheritdoc />
    public async ValueTask<ReadinessResult> CheckAsync(ReadinessContext context, CancellationToken cancellationToken = default)
    {
        var tenantId = context.TenantId;
        if (string.IsNullOrWhiteSpace(tenantId))
            return ReadinessResult.Healthy(TenantLifecycleReasons.Unknown);
        var snapshot = await _store.GetStatusAsync(TenantLifecycleOperationId.Parse(tenantId), cancellationToken).ConfigureAwait(false);
        if (snapshot is null)
            return ReadinessResult.Healthy(TenantLifecycleReasons.Unknown);
        return snapshot.State switch
        {
            TenantLifecycleOperationState.Succeeded => ReadinessResult.Healthy(snapshot.State.ToReason()),
            TenantLifecycleOperationState.Running => ReadinessResult.Healthy(snapshot.State.ToReason()),
            TenantLifecycleOperationState.Pending => ReadinessResult.Healthy(snapshot.State.ToReason()),
            TenantLifecycleOperationState.Retryable => ReadinessResult.Unhealthy(snapshot.State.ToReason(), snapshot.SafeFailureMessage),
            TenantLifecycleOperationState.PermanentlyFailed => ReadinessResult.Unhealthy(snapshot.State.ToReason(), snapshot.SafeFailureMessage),
            TenantLifecycleOperationState.PolicyDenied => ReadinessResult.Unhealthy(snapshot.State.ToReason(), snapshot.SafeFailureMessage),
            TenantLifecycleOperationState.Canceled => ReadinessResult.Unhealthy(snapshot.State.ToReason(), snapshot.SafeFailureMessage),
            _ => ReadinessResult.Unhealthy(TenantLifecycleReasons.Unknown, snapshot.SafeFailureMessage),
        };
    }
}

/// <summary>Provider-neutral readiness surface the platform exposes. The HTTP readiness host adapts this contract.</summary>
public interface IReadinessCheck
{
    /// <summary>Returns the readiness result for the supplied context.</summary>
    ValueTask<ReadinessResult> CheckAsync(ReadinessContext context, CancellationToken cancellationToken = default);
}

/// <summary>Context passed to a readiness check.</summary>
public sealed record ReadinessContext(string? TenantId);

/// <summary>Readiness outcome. <see cref="Reason"/> is a stable, secret-free diagnostic.</summary>
public sealed record ReadinessResult(bool IsHealthy, string Reason, string? Detail = null)
{
    /// <summary>Creates a healthy result with a reason.</summary>
    public static ReadinessResult Healthy(string reason) => new(true, reason);
    /// <summary>Creates an unhealthy result with a reason and optional safe detail.</summary>
    public static ReadinessResult Unhealthy(string reason, string? detail = null) => new(false, reason, detail);
}
