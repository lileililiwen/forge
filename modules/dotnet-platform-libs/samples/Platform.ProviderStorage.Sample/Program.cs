using Microsoft.Extensions.DependencyInjection;
using Platform.Storage.Contracts;
using Platform.Storage.Keys;
using Platform.Storage.Local;

// Stage 5: provider adapter. The application selects the local
// filesystem adapter and owns the root path, retention, and
// authorization. No credentials exist anywhere in this stage.
var root = args.FirstOrDefault() ?? Path.Combine(Path.GetTempPath(), "platform-storage-sample");
var services = new ServiceCollection();
// The application owns the root path, so it constructs the adapter with an
// explicit factory instead of container activation.
services.AddSingleton<IObjectStorage>(new LocalFileStorage(root));
await using var provider = services.BuildServiceProvider();
var storage = provider.GetRequiredService<IObjectStorage>();
var providerStatus = (IStorageProviderStatus)storage;
var key = new StorageObjectKey("matrix/hello.txt");
var payload = "provider-adapter-sample"u8.ToArray();
await using (var content = new MemoryStream(payload, writable: false))
{
    var upload = await storage.UploadAsync(new StorageUploadRequest(key, content, "text/plain", payload.Length));
    Console.WriteLine($"upload={upload.Status} provider={providerStatus.Status}");
}

var download = await storage.DownloadAsync(key);
await using (var content = download.Value!)
{
    using var reader = new StreamReader(content.Content);
    Console.WriteLine($"download={await reader.ReadToEndAsync()}");
}

await storage.DeleteAsync(key);
Console.WriteLine("cleanup=deleted");
