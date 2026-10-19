namespace Platform.Auditing.Contracts;

/// <summary>
/// Marks an entity type as opted-in to EF Core audit change capture. Only entities implementing
/// this interface are inspected by the auditing interceptor; product types opt in explicitly.
/// </summary>
public interface IAuditedEntity
{
}
