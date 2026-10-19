using Platform.Core.Context;

namespace Platform.Realtime.Tenant;

/// <summary>
/// Describes a candidate delivery route for a realtime message. The platform
/// asks the application whether the caller may deliver to the target tenant
/// before broadcasting any connection data.
/// </summary>
public sealed class RealtimeTenantRoute
{
    /// <summary>
    /// Gets or sets the tenant the message targets. <c>null</c> means a
    /// broadcast (every authorized connection) rather than a specific tenant.
    /// </summary>
    public string? TargetTenantId { get; set; }

    /// <summary>Gets or sets the caller attempting the delivery.</summary>
    public CallerContext Caller { get; set; } = CallerContext.Anonymous;
}
