namespace Platform.Auditing.Contracts;

/// <summary>The default provider status source reporting an always-available in-memory sink.</summary>
public sealed class DefaultAuditProviderStatusSource : IAuditProviderStatusSource
{
    /// <summary>The default sink name reported by this source.</summary>
    public const string DefaultName = "memory";

    /// <inheritdoc />
    public AuditProviderStatus GetStatus() => new(DefaultName, Available: true);
}
