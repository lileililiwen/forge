namespace Platform.Caching.Redis;

/// <summary>Application-owned serialization boundary for Redis values.</summary>
public interface ICacheValueSerializer
{
    /// <summary>Serializes a value without embedding serializer choice in cache contracts.</summary>
    byte[] Serialize<T>(T value);

    /// <summary>Deserializes a value or returns <c>null</c> for an incompatible payload.</summary>
    T? Deserialize<T>(byte[] payload);
}
