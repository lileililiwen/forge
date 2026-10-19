using Platform.Storage.Contracts;
using Platform.Storage.Keys;
using Platform.Storage.Local;

namespace Platform.SampleMatrix.Tests;

/// <summary>Stage 5: deterministic object storage through the local adapter without credentials.</summary>
public sealed class ProviderStorageTests : IDisposable
{
    private readonly string _root;

    /// <summary>Creates an isolated storage root per test run.</summary>
    public ProviderStorageTests()
    {
        _root = Path.Combine(Path.GetTempPath(), $"matrix-storage-{Guid.NewGuid():N}");
    }

    /// <inheritdoc />
    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }

        GC.SuppressFinalize(this);
    }

    /// <summary>Verifies upload, download, metadata, and delete round-trip locally.</summary>
    [Fact]
    public async Task Objects_round_trip_through_local_adapter()
    {
        var storage = (IObjectStorage)new LocalFileStorage(_root);
        var key = new StorageObjectKey("matrix/hello.txt");
        var payload = "matrix-payload"u8.ToArray();

        await using (var content = new MemoryStream(payload, writable: false))
        {
            var upload = await storage.UploadAsync(new StorageUploadRequest(key, content, "text/plain", payload.Length));
            Assert.Equal(StorageOutcomeStatus.Succeeded, upload.Status);
        }

        Assert.Equal(StorageProviderState.Healthy, ((IStorageProviderStatus)storage).Status.State);

        var download = await storage.DownloadAsync(key);
        Assert.Equal(StorageOutcomeStatus.Succeeded, download.Status);
        await using (var content = download.Value!)
        {
            using var reader = new StreamReader(content.Content);
            Assert.Equal("matrix-payload", await reader.ReadToEndAsync());
        }

        var metadata = await storage.GetMetadataAsync(key);
        Assert.NotNull(metadata);
        Assert.Equal(payload.Length, metadata.LengthBytes);

        Assert.Equal(StorageOutcomeStatus.Succeeded, (await storage.DeleteAsync(key)).Status);
        Assert.Equal(StorageOutcomeStatus.NotFound, (await storage.DownloadAsync(key)).Status);
    }
}
