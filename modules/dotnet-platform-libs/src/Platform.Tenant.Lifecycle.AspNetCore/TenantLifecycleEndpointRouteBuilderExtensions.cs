using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.Tenant.Lifecycle.AspNetCore;

/// <summary>Provider-neutral status endpoints for tenant lifecycle.</summary>
public static class TenantLifecycleEndpointRouteBuilderExtensions
{
    /// <summary>Maps the status endpoint for a single operation. Returns the latest <see cref="TenantLifecycleOperationStatus"/>.</summary>
    public static RouteHandlerBuilder MapPlatformTenantLifecycleStatus(this IEndpointRouteBuilder endpoints, string path = "/platform/tenant-lifecycle/operations/{operationId}")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapGet(path, async (string operationId, ITenantLifecycleStore store, CancellationToken cancellationToken) =>
        {
            if (string.IsNullOrWhiteSpace(operationId))
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "operationId is required.");
            var status = await store.GetStatusAsync(TenantLifecycleOperationId.Parse(operationId), cancellationToken).ConfigureAwait(false);
            return status is null
                ? Results.NotFound()
                : Results.Ok(status);
        });
    }

    /// <summary>Maps the operator resume endpoint. Returns the latest status after a resume attempt.</summary>
    public static RouteHandlerBuilder MapPlatformTenantLifecycleResume(this IEndpointRouteBuilder endpoints, string path = "/platform/tenant-lifecycle/operations/{operationId}/resume")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapPost(path, async (string operationId, ITenantLifecycleOrchestrator orchestrator, CancellationToken cancellationToken) =>
        {
            if (string.IsNullOrWhiteSpace(operationId))
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "operationId is required.");
            try
            {
                var status = await orchestrator.ResumeAsync(TenantLifecycleOperationId.Parse(operationId), cancellationToken).ConfigureAwait(false);
                return Results.Ok(status);
            }
            catch (InvalidOperationException exception)
            {
                return Results.Problem(statusCode: StatusCodes.Status409Conflict, title: "Resume refused", detail: exception.Message);
            }
        });
    }
}
