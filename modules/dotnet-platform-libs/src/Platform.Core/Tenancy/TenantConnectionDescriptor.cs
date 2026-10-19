namespace Platform.Core.Tenancy;

/// <summary>
/// Describes the database connection that the EF Core adapter should
/// use for a given tenant. The application owns the connection
/// strings; the platform only routes the descriptor through the
/// scoped connection provider so transactions can be shared across
/// participating <c>DbContext</c> instances.
/// </summary>
public sealed record TenantConnectionDescriptor(
    string Name,
    string ConnectionString,
    string ProviderName)
{
    /// <summary>Builds the shared-connection descriptor used when the resolver does not select a dedicated database.</summary>
    public static TenantConnectionDescriptor Shared(string connectionString, string providerName) =>
        new("shared", connectionString, providerName);
}
