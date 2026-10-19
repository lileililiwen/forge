namespace Platform.Authorization;

/// <summary>A module-owned resource/action permission.</summary>
public sealed record PermissionDefinition(string Resource, string Action)
{
    /// <summary>Gets the stable permission key.</summary>
    public string Key => Resource + "." + Action;
}

/// <summary>Catalog of permissions registered by consuming modules.</summary>
public sealed class PermissionCatalog
{
    private readonly Dictionary<string, PermissionDefinition> definitions = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Registers a permission and rejects duplicate keys.</summary>
    public PermissionCatalog Register(PermissionDefinition permission)
    {
        ArgumentNullException.ThrowIfNull(permission);
        if (string.IsNullOrWhiteSpace(permission.Resource) || string.IsNullOrWhiteSpace(permission.Action))
            throw new ArgumentException("Permission resource and action are required.", nameof(permission));
        if (!definitions.TryAdd(permission.Key, permission))
            throw new InvalidOperationException("Permission is already registered: " + permission.Key);
        return this;
    }

    /// <summary>Gets all registered permissions.</summary>
    public IReadOnlyCollection<PermissionDefinition> All => definitions.Values.ToArray();
    /// <summary>Finds a registered permission by key.</summary>
    public PermissionDefinition? Find(string key) => key is null ? null : definitions.GetValueOrDefault(key);
}

/// <summary>Stable policy naming helpers.</summary>
public static class PlatformPolicyNames
{
    /// <summary>Creates a policy name for a permission key.</summary>
    public static string ForPermission(string permissionKey) => "platform:permission:" + (permissionKey ?? throw new ArgumentNullException(nameof(permissionKey)));
    /// <summary>Creates a policy name for a role.</summary>
    public static string ForRole(string role) => "platform:role:" + (role ?? throw new ArgumentNullException(nameof(role)));
}

/// <summary>Normalized authorization decision.</summary>
public sealed record AuthorizationDecision(bool Succeeded, string? Requirement = null, string? Reason = null)
{
    /// <summary>Creates a granted decision.</summary>
    public static AuthorizationDecision Granted(string? requirement = null) => new(true, requirement);
    /// <summary>Creates a denied decision.</summary>
    public static AuthorizationDecision Denied(string requirement, string? reason = null) => new(false, requirement, reason);
}

/// <summary>Receives authorization decisions for diagnostics or audit.</summary>
public interface IAuthorizationDecisionAuditor
{
    /// <summary>Records a decision.</summary>
    ValueTask RecordAsync(AuthorizationDecision decision, string? subjectId, CancellationToken cancellationToken = default);
}
