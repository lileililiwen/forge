#pragma warning disable CS1591
using Platform.Ai.Contracts;

namespace Platform.Ai.Testing;

public sealed class FakeAiProvider : IAiTextProvider
{
    private readonly IAiUsageSink? _usage;
    public FakeAiProvider(IAiUsageSink? usage = null) => _usage = usage;
    public AiProviderName Name => AiProviderName.Create("fake");
    public AiProviderCapabilities Capabilities { get; } = new(new HashSet<string>(StringComparer.OrdinalIgnoreCase) { "text", "structured-output", "streaming" });
    public async ValueTask<AiGenerationResult> GenerateAsync(AiTextRequest request, CancellationToken cancellationToken = default)
    {
        var result = new AiGenerationResult($"fake:{request.Feature.Value}", new AiUsage(request.Messages.Sum(m => m.Content.Length), 1, request.Messages.Sum(m => m.Content.Length) + 1), Model: request.Model ?? "fake-model");
        if (_usage is not null) await _usage.RecordAsync(request.Feature, result.Usage, cancellationToken).ConfigureAwait(false);
        return result;
    }
    public async IAsyncEnumerable<AiStreamChunk> StreamAsync(AiTextRequest request, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        yield return new AiStreamChunk($"fake:{request.Feature.Value}", true, new AiUsage(0, 1, 1));
        await Task.CompletedTask;
    }
}

public sealed class RecordingAiProvider : IAiTextProvider
{
    private readonly IAiTextProvider _inner;
    public RecordingAiProvider(IAiTextProvider inner) => _inner = inner ?? throw new ArgumentNullException(nameof(inner));
    public int GenerationCalls { get; private set; }
    public AiProviderName Name => _inner.Name;
    public AiProviderCapabilities Capabilities => _inner.Capabilities;
    public ValueTask<AiGenerationResult> GenerateAsync(AiTextRequest request, CancellationToken cancellationToken = default) { GenerationCalls++; return _inner.GenerateAsync(request, cancellationToken); }
    public IAsyncEnumerable<AiStreamChunk> StreamAsync(AiTextRequest request, CancellationToken cancellationToken = default) => _inner.StreamAsync(request, cancellationToken);
}

public sealed class RecordingUsageSink : IAiUsageSink
{
    public List<(AiFeatureKey Feature, AiUsage Usage)> Entries { get; } = [];
    public ValueTask RecordAsync(AiFeatureKey feature, AiUsage usage, CancellationToken cancellationToken = default) { Entries.Add((feature, usage)); return ValueTask.CompletedTask; }
}
