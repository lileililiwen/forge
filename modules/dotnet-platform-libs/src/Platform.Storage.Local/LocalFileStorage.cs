using Platform.Storage.Contracts;
using Platform.Storage.Keys;

namespace Platform.Storage.Local;

/// <summary>Local filesystem object storage for development and single-host deployments.</summary>
public sealed class LocalFileStorage : IObjectStorage, IStorageProviderStatus
{
    private readonly string _root;
    private readonly StorageOptions _options;

    /// <summary>Creates a local store rooted at the supplied directory.</summary>
    public LocalFileStorage(string rootPath, StorageOptions? options = null)
    {
        if (string.IsNullOrWhiteSpace(rootPath)) throw new ArgumentException("A root path is required.", nameof(rootPath));
        _root = Path.GetFullPath(rootPath);
        _options = options ?? new StorageOptions();
        _options.Validate();
        Directory.CreateDirectory(_root);
    }

    /// <inheritdoc />
    public StorageProviderStatus Status => new("local", StorageProviderState.Healthy);

    /// <inheritdoc />
    public async Task<StorageOutcome> UploadAsync(StorageUploadRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        request.Validate(_options);
        var destination = Resolve(request.Key);
        Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
        var temporary = destination + "." + Guid.NewGuid().ToString("N") + ".tmp";
        try
        {
            await using (var output = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None, 81920, FileOptions.Asynchronous | FileOptions.SequentialScan))
            {
                await request.Content.CopyToAsync(output, 81920, cancellationToken).ConfigureAwait(false);
                if (output.Length != request.ContentLength || output.Length > _options.MaximumObjectBytes)
                    return new StorageOutcome(StorageOutcomeStatus.Rejected, new StorageFailure("storage.size_limit", "The upload size is invalid or exceeds the configured limit.", false));
                await output.FlushAsync(cancellationToken).ConfigureAwait(false);
            }
            File.Move(temporary, destination, overwrite: true);
            return StorageOutcome.Succeeded();
        }
        catch (OperationCanceledException) { throw; }
        catch (IOException) { return Unavailable(); }
        finally { if (File.Exists(temporary)) File.Delete(temporary); }
    }

    /// <inheritdoc />
    public async Task<StorageDownloadResult> DownloadAsync(StorageObjectKey key, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var path = Resolve(key);
        if (!File.Exists(path)) return StorageDownloadResult.NotFound();
        var info = new FileInfo(path);
        var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read, 81920, FileOptions.Asynchronous | FileOptions.SequentialScan);
        await Task.CompletedTask.ConfigureAwait(false);
        return StorageDownloadResult.Succeeded(new StorageReadResult(stream, new StorageObjectMetadata(info.Length, ContentType(path), new DateTimeOffset(info.LastWriteTimeUtc, TimeSpan.Zero), null)));
    }

    /// <inheritdoc />
    public Task<StorageObjectMetadata?> GetMetadataAsync(StorageObjectKey key, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var path = Resolve(key);
        if (!File.Exists(path)) return Task.FromResult<StorageObjectMetadata?>(null);
        var info = new FileInfo(path);
        return Task.FromResult<StorageObjectMetadata?>(new StorageObjectMetadata(info.Length, ContentType(path), new DateTimeOffset(info.LastWriteTimeUtc, TimeSpan.Zero), null));
    }

    /// <inheritdoc />
    public Task<StorageOutcome> DeleteAsync(StorageObjectKey key, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var path = Resolve(key);
        if (!File.Exists(path)) return Task.FromResult(StorageOutcome.NotFound());
        try { File.Delete(path); return Task.FromResult(StorageOutcome.Succeeded()); }
        catch (IOException) { return Task.FromResult(Unavailable()); }
    }

    /// <inheritdoc />
    public Task<(StorageOutcome Outcome, PresignedOperation? Operation)> PresignAsync(PresignRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        request.Validate(_options);
        cancellationToken.ThrowIfCancellationRequested();
        var encoded = Convert.ToBase64String(System.Text.Encoding.UTF8.GetBytes(request.Key.Value));
        var uri = new Uri($"local://{request.Operation.ToString().ToLowerInvariant()}/{encoded}");
        return Task.FromResult<(StorageOutcome, PresignedOperation?)>((StorageOutcome.Succeeded(), new PresignedOperation(uri, request.Key, request.Operation, DateTimeOffset.UtcNow.Add(request.Lifetime), request.ContentType, request.MaximumBytes)));
    }

    private string Resolve(StorageObjectKey key)
    {
        var path = Path.GetFullPath(Path.Combine(_root, key.Value.Replace('/', Path.DirectorySeparatorChar)));
        if (!path.StartsWith(_root + Path.DirectorySeparatorChar, StringComparison.Ordinal) && !string.Equals(path, _root, StringComparison.Ordinal)) throw new ArgumentException("The object key resolves outside the storage root.", nameof(key));
        return path;
    }
    private static string ContentType(string path) => Path.GetExtension(path).ToLowerInvariant() switch { ".txt" => "text/plain", ".json" => "application/json", ".jpg" or ".jpeg" => "image/jpeg", ".png" => "image/png", ".pdf" => "application/pdf", _ => "application/octet-stream" };
    private static StorageOutcome Unavailable() => StorageOutcome.Unavailable(new StorageFailure("storage.provider_unavailable", "The storage provider is unavailable.", true));
}
