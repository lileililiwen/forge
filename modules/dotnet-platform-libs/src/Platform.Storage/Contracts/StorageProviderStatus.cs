namespace Platform.Storage.Contracts;

/// <summary>Provider status exposed for readiness integration.</summary>
public sealed record StorageProviderStatus(string Provider, StorageProviderState State, string? Code = null);

/// <summary>Storage provider availability.</summary>
public enum StorageProviderState
{
    /// <summary>Provider is available.</summary>
    Healthy,
    /// <summary>Provider is unavailable.</summary>
    Unavailable
}

/// <summary>Exposes safe provider health.</summary>
public interface IStorageProviderStatus
{
    /// <summary>Current provider status.</summary>
    StorageProviderStatus Status { get; }
}
