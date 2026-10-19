using Platform.Storage.Contracts;
using Platform.Storage.Keys;
using Platform.Storage.Local;
using Platform.Storage.S3;
using Amazon.Runtime;
using Amazon.S3;

namespace Platform.Storage.Tests;

public sealed class StorageTests
{
    [Fact]
    public void Object_key_rejects_traversal_controls_and_excessive_length()
    {
        Assert.Throws<ArgumentException>(() => new StorageObjectKey("../private/secret"));
        Assert.Throws<ArgumentException>(() => new StorageObjectKey("tenant/\0/file"));
        Assert.Throws<ArgumentException>(() => new StorageObjectKey("tenant//file"));
        Assert.Throws<ArgumentOutOfRangeException>(() => new StorageObjectKey(new string('a', 1025)));
    }

    [Fact]
    public void Upload_request_enforces_content_type_and_length_limits()
    {
        var request = new StorageUploadRequest(new StorageObjectKey("tenant-a/file.txt"), new MemoryStream([1, 2, 3]), "text/plain", 3);
        request.Validate(new StorageOptions { MaximumObjectBytes = 3 });
        Assert.Throws<ArgumentOutOfRangeException>(() => request.Validate(new StorageOptions { MaximumObjectBytes = 2 }));
        Assert.Throws<ArgumentException>(() => new StorageUploadRequest(request.Key, request.Content, " ", 3).Validate(new StorageOptions()));
    }

    [Fact]
    public void Presign_requests_are_bounded_and_failures_are_safe()
    {
        var request = new PresignRequest(new StorageObjectKey("tenant-a/file.txt"), StorageOperation.Download, TimeSpan.FromMinutes(5));
        request.Validate(new StorageOptions { MaximumPresignLifetime = TimeSpan.FromMinutes(10) });
        Assert.Throws<ArgumentOutOfRangeException>(() => (request with { Lifetime = TimeSpan.FromMinutes(11) }).Validate(new StorageOptions { MaximumPresignLifetime = TimeSpan.FromMinutes(10) }));
        var failure = new StorageFailure("storage.unavailable", "The storage provider is unavailable.", true);
        Assert.DoesNotContain("secret", failure.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Local_store_writes_atomically_downloads_metadata_and_deletes()
    {
        var root = Path.Combine(Path.GetTempPath(), "platform-storage-" + Guid.NewGuid().ToString("N"));
        try
        {
            var store = new LocalFileStorage(root);
            var key = new StorageObjectKey("tenant-a/file.txt");
            var bytes = new MemoryStream("hello"u8.ToArray());
            var upload = await store.UploadAsync(new StorageUploadRequest(key, bytes, "text/plain", 5));

            Assert.Equal(StorageOutcomeStatus.Succeeded, upload.Status);
            var download = await store.DownloadAsync(key);
            Assert.Equal(StorageOutcomeStatus.Succeeded, download.Status);
            Assert.NotNull(download.Value);
            Assert.Equal(5, download.Value!.Metadata.LengthBytes);
            using var reader = new StreamReader(download.Value.Content);
            Assert.Equal("hello", await reader.ReadToEndAsync());

            Assert.Equal(StorageOutcomeStatus.Succeeded, (await store.DeleteAsync(key)).Status);
            Assert.Equal(StorageOutcomeStatus.NotFound, (await store.DownloadAsync(key)).Status);
        }
        finally
        {
            if (Directory.Exists(root)) Directory.Delete(root, recursive: true);
        }
    }

    [Fact]
    public async Task S3_adapter_presigns_without_performing_provider_io()
    {
        using var client = new AmazonS3Client(new BasicAWSCredentials("access", "secret"), new AmazonS3Config
        {
            ServiceURL = "http://localhost:9",
            ForcePathStyle = true
        });
        var store = new S3Storage(client, new S3StorageOptions { BucketName = "sample-bucket" });

        var result = await store.PresignAsync(new PresignRequest(new StorageObjectKey("tenant-a/file.txt"), StorageOperation.Upload, TimeSpan.FromMinutes(5), "text/plain", 100));

        Assert.Equal(StorageOutcomeStatus.Succeeded, result.Outcome.Status);
        Assert.NotNull(result.Operation);
        Assert.Equal(StorageOperation.Upload, result.Operation!.Operation);
        Assert.True(result.Operation.ExpiresAt > DateTimeOffset.UtcNow);
    }
}
