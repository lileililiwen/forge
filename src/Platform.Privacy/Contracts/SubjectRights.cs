namespace Platform.Privacy.Contracts;

/// <summary>One provider's contribution to a subject export: named documents.</summary>
/// <param name="Provider">Provider name.</param>
/// <param name="Documents">Exported documents, keyed by application-defined name.</param>
public sealed record SubjectExport(string Provider, IReadOnlyDictionary<string, string> Documents);

/// <summary>Aggregated export bundle with per-provider failures preserved.</summary>
/// <param name="SubjectId">Subject the bundle was built for.</param>
/// <param name="Exports">Successful provider contributions.</param>
/// <param name="Failures">Provider name to error; successes are never dropped for these.</param>
public sealed record ExportBundle(
    string SubjectId,
    IReadOnlyList<SubjectExport> Exports,
    IReadOnlyDictionary<string, string> Failures);

/// <summary>One provider's erasure report.</summary>
/// <param name="Provider">Provider name.</param>
/// <param name="ErasedItems">Items erased or anonymized.</param>
public sealed record ProviderErasure(string Provider, long ErasedItems);

/// <summary>Aggregated erasure report.</summary>
/// <param name="SubjectId">Subject the report covers.</param>
/// <param name="Providers">Per-provider reports.</param>
/// <param name="Failures">Provider name to error.</param>
public sealed record ErasureReport(
    string SubjectId,
    IReadOnlyList<ProviderErasure> Providers,
    IReadOnlyDictionary<string, string> Failures);

/// <summary>Application-owned source of a subject's exportable data.</summary>
public interface ISubjectExportProvider
{
    /// <summary>Provider name, used in bundles and failure reports.</summary>
    string Name { get; }

    /// <summary>Exports the subject's data held by this provider.</summary>
    Task<SubjectExport> ExportAsync(string subjectId, CancellationToken cancellationToken = default);
}

/// <summary>Application-owned eraser of a subject's data.</summary>
public interface ISubjectErasureProvider
{
    /// <summary>Provider name, used in reports and failure records.</summary>
    string Name { get; }

    /// <summary>Erases or anonymizes the subject's data held by this provider.</summary>
    Task<ProviderErasure> EraseAsync(string subjectId, CancellationToken cancellationToken = default);
}

/// <summary>Registry applications populate; aggregates without interpreting.</summary>
public sealed class SubjectRightsRegistry
{
    private readonly List<ISubjectExportProvider> _exportProviders = new();
    private readonly List<ISubjectErasureProvider> _erasureProviders = new();

    /// <summary>Registers an export provider.</summary>
    public SubjectRightsRegistry AddExport(ISubjectExportProvider provider)
    {
        ArgumentNullException.ThrowIfNull(provider);
        _exportProviders.Add(provider);
        return this;
    }

    /// <summary>Registers an erasure provider.</summary>
    public SubjectRightsRegistry AddErasure(ISubjectErasureProvider provider)
    {
        ArgumentNullException.ThrowIfNull(provider);
        _erasureProviders.Add(provider);
        return this;
    }

    /// <summary>Runs every export provider; one failure never drops other successes.</summary>
    public async Task<ExportBundle> ExportAsync(string subjectId, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(subjectId);
        var exports = new List<SubjectExport>();
        var failures = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var provider in _exportProviders)
        {
            try
            {
                exports.Add(await provider.ExportAsync(subjectId, cancellationToken).ConfigureAwait(false));
            }
            catch (Exception ex)
            {
                failures[provider.Name] = ex.Message;
            }
        }

        return new ExportBundle(subjectId, exports, failures);
    }

    /// <summary>Runs every erasure provider and aggregates the counts unchanged.</summary>
    public async Task<ErasureReport> EraseAsync(string subjectId, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(subjectId);
        var reports = new List<ProviderErasure>();
        var failures = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var provider in _erasureProviders)
        {
            try
            {
                reports.Add(await provider.EraseAsync(subjectId, cancellationToken).ConfigureAwait(false));
            }
            catch (Exception ex)
            {
                failures[provider.Name] = ex.Message;
            }
        }

        return new ErasureReport(subjectId, reports, failures);
    }
}
