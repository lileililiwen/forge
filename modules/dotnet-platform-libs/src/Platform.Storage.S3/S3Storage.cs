using Amazon;
using Amazon.Runtime;
using Amazon.S3;
using Amazon.S3.Model;
using Platform.Storage.Contracts;
using Platform.Storage.Keys;

namespace Platform.Storage.S3;

/// <summary>S3-compatible object storage adapter using an application-provided client.</summary>
public sealed class S3Storage : IObjectStorage, IStorageProviderStatus
{
    private readonly IAmazonS3 _client;
    private readonly S3StorageOptions _options;
    private volatile StorageProviderStatus _status;

    /// <summary>Creates an adapter over an application-owned S3 client.</summary>
    public S3Storage(IAmazonS3 client, S3StorageOptions options)
    {
        _client = client ?? throw new ArgumentNullException(nameof(client));
        _options = options ?? throw new ArgumentNullException(nameof(options));
        _options.Validate();
        _status = new StorageProviderStatus("s3", StorageProviderState.Healthy);
    }

    /// <inheritdoc />
    public StorageProviderStatus Status => _status;

    /// <inheritdoc />
    public async Task<StorageOutcome> UploadAsync(StorageUploadRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        request.Validate(_options.Limits);
        try
        {
            await Wait(_client.PutObjectAsync(new PutObjectRequest { BucketName = _options.BucketName, Key = Physical(request.Key), InputStream = request.Content, ContentType = request.ContentType }, cancellationToken), cancellationToken).ConfigureAwait(false);
            _status = new StorageProviderStatus("s3", StorageProviderState.Healthy);
            return StorageOutcome.Succeeded();
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (Exception exception) when (IsProviderFailure(exception)) { return Unavailable(); }
    }

    /// <inheritdoc />
    public async Task<StorageDownloadResult> DownloadAsync(StorageObjectKey key, CancellationToken cancellationToken = default)
    {
        try
        {
            var response = await Wait(_client.GetObjectAsync(new GetObjectRequest { BucketName = _options.BucketName, Key = Physical(key) }, cancellationToken), cancellationToken).ConfigureAwait(false);
            var modified = response.LastModified ?? DateTime.UtcNow;
            var metadata = new StorageObjectMetadata(response.ContentLength, string.IsNullOrWhiteSpace(response.Headers.ContentType) ? "application/octet-stream" : response.Headers.ContentType, new DateTimeOffset(DateTime.SpecifyKind(modified, DateTimeKind.Utc), TimeSpan.Zero), response.ETag);
            return StorageDownloadResult.Succeeded(new StorageReadResult(response.ResponseStream, metadata));
        }
        catch (AmazonS3Exception exception) when (exception.StatusCode == System.Net.HttpStatusCode.NotFound) { return StorageDownloadResult.NotFound(); }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (Exception exception) when (IsProviderFailure(exception)) { var outcome = Unavailable(); return StorageDownloadResult.Unavailable(outcome.Failure!); }
    }

    /// <inheritdoc />
    public async Task<StorageObjectMetadata?> GetMetadataAsync(StorageObjectKey key, CancellationToken cancellationToken = default)
    {
        try
        {
            var response = await Wait(_client.GetObjectMetadataAsync(new GetObjectMetadataRequest { BucketName = _options.BucketName, Key = Physical(key) }, cancellationToken), cancellationToken).ConfigureAwait(false);
            var modified = response.LastModified ?? DateTime.UtcNow;
            return new StorageObjectMetadata(response.ContentLength, string.IsNullOrWhiteSpace(response.Headers.ContentType) ? "application/octet-stream" : response.Headers.ContentType, new DateTimeOffset(DateTime.SpecifyKind(modified, DateTimeKind.Utc), TimeSpan.Zero), response.ETag);
        }
        catch (AmazonS3Exception exception) when (exception.StatusCode == System.Net.HttpStatusCode.NotFound) { return null; }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (Exception exception) when (IsProviderFailure(exception)) { _ = Unavailable(); return null; }
    }

    /// <inheritdoc />
    public async Task<StorageOutcome> DeleteAsync(StorageObjectKey key, CancellationToken cancellationToken = default)
    {
        try
        {
            await Wait(_client.DeleteObjectAsync(new DeleteObjectRequest { BucketName = _options.BucketName, Key = Physical(key) }, cancellationToken), cancellationToken).ConfigureAwait(false);
            return StorageOutcome.Succeeded();
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (Exception exception) when (IsProviderFailure(exception)) { return Unavailable(); }
    }

    /// <inheritdoc />
    public async Task<(StorageOutcome Outcome, PresignedOperation? Operation)> PresignAsync(PresignRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        request.Validate(_options.Limits);
        try
        {
            var expiresAt = DateTimeOffset.UtcNow.Add(request.Lifetime);
            var presign = new GetPreSignedUrlRequest { BucketName = _options.BucketName, Key = Physical(request.Key), Verb = request.Operation == StorageOperation.Upload ? HttpVerb.PUT : HttpVerb.GET, Expires = expiresAt.UtcDateTime };
            if (request.ContentType is not null) presign.ContentType = request.ContentType;
            var url = await Wait(_client.GetPreSignedURLAsync(presign), cancellationToken).ConfigureAwait(false);
            return (StorageOutcome.Succeeded(), new PresignedOperation(new Uri(url), request.Key, request.Operation, expiresAt, request.ContentType, request.MaximumBytes));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (Exception exception) when (IsProviderFailure(exception)) { return (Unavailable(), null); }
    }

    private string Physical(StorageObjectKey key) => string.IsNullOrEmpty(_options.KeyPrefix) ? key.Value : _options.KeyPrefix.TrimEnd('/') + "/" + key.Value;
    private static bool IsProviderFailure(Exception exception) => exception is AmazonS3Exception or AmazonServiceException or TimeoutException or HttpRequestException;
    private StorageOutcome Unavailable()
    {
        _status = new StorageProviderStatus("s3", StorageProviderState.Unavailable, "storage.provider_unavailable");
        return StorageOutcome.Unavailable(new StorageFailure("storage.provider_unavailable", "The storage provider is unavailable.", true));
    }
    private async Task<T> Wait<T>(Task<T> task, CancellationToken cancellationToken) => await task.WaitAsync(_options.Limits.OperationTimeout, cancellationToken).ConfigureAwait(false);
}
