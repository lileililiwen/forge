using Microsoft.Extensions.Options;

namespace Platform.Realtime;

/// <summary>
/// Bounded-connection configuration shared by every realtime transport.
/// All values are safe by default. Consumers override only what they need.
/// </summary>
public sealed class RealtimeConnectionOptions
{
    /// <summary>
    /// The configuration section name bound by
    /// <c>Platform.Realtime.DependencyInjection.ServiceCollectionExtensions.AddPlatformRealtime</c>.
    /// </summary>
    public const string SectionName = "Realtime";

    /// <summary>Gets or sets the maximum number of concurrent realtime connections. Defaults to <c>1000</c>.</summary>
    public int MaxConcurrentConnections { get; set; } = 1000;

    /// <summary>Gets or sets the maximum accepted payload size in bytes for a single message. Defaults to <c>65536</c> (64 KiB).</summary>
    public int MaxPayloadBytes { get; set; } = 64 * 1024;

    /// <summary>Gets or sets the idle timeout after which an inactive connection is released. Defaults to five minutes.</summary>
    public TimeSpan ConnectionIdleTimeout { get; set; } = TimeSpan.FromMinutes(5);

    /// <summary>Gets or sets the interval at which a heartbeat comment is emitted to keep a streaming connection alive. Defaults to 30 seconds.</summary>
    public TimeSpan HeartbeatInterval { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>
    /// Gets or sets whether a message without an explicit target tenant may be
    /// delivered to connections outside the caller's tenant. Defaults to
    /// <c>false</c>; cross-tenant broadcast requires an explicit application
    /// authorization decision.
    /// </summary>
    public bool AllowCrossTenantBroadcast { get; set; }

    /// <summary>
    /// Validates the option values and returns human-readable failures.
    /// </summary>
    /// <returns>A non-empty list when one or more values are invalid.</returns>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (MaxConcurrentConnections is < 1 or > 100_000)
            errors.Add("MaxConcurrentConnections must be between 1 and 100000.");
        if (MaxPayloadBytes is < 1 or > 10 * 1024 * 1024)
            errors.Add("MaxPayloadBytes must be between 1 and 10485760 (10 MiB).");
        if (ConnectionIdleTimeout <= TimeSpan.Zero)
            errors.Add("ConnectionIdleTimeout must be a positive span.");
        if (HeartbeatInterval <= TimeSpan.Zero)
            errors.Add("HeartbeatInterval must be a positive span.");
        return errors;
    }
}
