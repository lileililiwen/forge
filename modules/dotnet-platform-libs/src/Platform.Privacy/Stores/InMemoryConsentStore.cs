using System.Collections.Concurrent;
using Platform.Privacy.Contracts;

namespace Platform.Privacy.Stores;

/// <summary>Thread-safe in-memory consent store for tests and single-instance products.</summary>
public sealed class InMemoryConsentStore : IConsentStore
{
    private readonly ConcurrentDictionary<string, ConsentRecord> _records = new(StringComparer.Ordinal);

    /// <inheritdoc />
    public Task<ConsentRecord?> GetAsync(string subjectId, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(subjectId);
        _records.TryGetValue(subjectId, out var record);
        return Task.FromResult(record);
    }

    /// <inheritdoc />
    public Task SetAsync(ConsentRecord record, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(record);
        _records[record.SubjectId] = record;
        return Task.CompletedTask;
    }
}
