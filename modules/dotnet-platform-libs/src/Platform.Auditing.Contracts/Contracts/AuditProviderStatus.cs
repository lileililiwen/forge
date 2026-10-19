namespace Platform.Auditing.Contracts;

/// <summary>Safe, schema-free health state for the configured audit sink.</summary>
/// <param name="Name">The sink name, for example <c>memory</c> or <c>sql</c>.</param>
/// <param name="Available">Whether the sink is currently able to accept events.</param>
/// <param name="Detail">Optional human-readable detail. Must not include secrets.</param>
public sealed record AuditProviderStatus(string Name, bool Available, string? Detail = null);

/// <summary>Reports the current health of the configured audit sink.</summary>
public interface IAuditProviderStatusSource
{
    /// <summary>Returns the current sink status.</summary>
    /// <returns>The sink status.</returns>
    AuditProviderStatus GetStatus();
}
