using Platform.Core.Context;

namespace Platform.Realtime.Authorization;

/// <summary>
/// Describes an inbound realtime connection attempt before it is accepted.
/// The platform never derives authorization from this data; applications
/// inspect it through <see cref="IRealtimeConnectionAuthorizer"/>.
/// </summary>
public sealed class RealtimeConnectionRequest
{
    /// <summary>Gets or sets the transport-assigned connection identifier.</summary>
    public string ConnectionId { get; set; } = string.Empty;

    /// <summary>Gets or sets the observed caller context (subject and tenant).</summary>
    public CallerContext Caller { get; set; } = CallerContext.Anonymous;

    /// <summary>
    /// Gets or sets optional transport metadata (headers, query hints). The
    /// platform does not interpret these values; they are supplied to the
    /// application authorizer for policy decisions.
    /// </summary>
    public IReadOnlyDictionary<string, string>? Metadata { get; set; }
}
