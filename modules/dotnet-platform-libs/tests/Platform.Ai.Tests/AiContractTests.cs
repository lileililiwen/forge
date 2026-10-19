using Platform.Ai;
using Platform.Ai.Contracts;
using Platform.Ai.Testing;

namespace Platform.Ai.Tests;

public sealed class AiContractTests
{
    [Fact]
    public async Task Policy_rejection_skips_provider_and_identifies_quota()
    {
        var provider = new RecordingAiProvider(new FakeAiProvider());
        var client = new AiClient(new SingleAiProviderRouter(provider), new RejectingPolicy());
        var result = await client.GenerateAsync(new AiTextRequest(AiFeatureKey.Create("summarize"), [new("user", "secret")]), CancellationToken.None);
        Assert.Equal(AiFailureCategory.QuotaExceeded, result.Failure?.Category);
        Assert.Equal(0, provider.GenerationCalls);
    }

    [Fact]
    public async Task Fake_provider_is_deterministic_and_usage_is_recorded_without_prompt_logging()
    {
        var recorder = new RecordingUsageSink();
        var result = await new FakeAiProvider(recorder).GenerateAsync(new AiTextRequest(AiFeatureKey.Create("draft"), [new("user", "hello")]), CancellationToken.None);
        Assert.Equal("fake:draft", result.Text);
        var entry = Assert.Single(recorder.Entries);
        Assert.DoesNotContain("hello", entry.ToString(), StringComparison.Ordinal);
    }

    [Fact]
    public async Task Structured_output_returns_explicit_unsupported_capability()
    {
        var provider = new RecordingAiProvider(new CapabilityLimitedProvider(new FakeAiProvider()));
        var client = new AiClient(new SingleAiProviderRouter(provider), new AllowAllAiFeaturePolicy());
        var result = await client.GenerateStructuredAsync(new AiStructuredRequest(new AiTextRequest(AiFeatureKey.Create("draft"), [new("user", "hello")]), "{}"));
        Assert.Equal(AiFailureCategory.UnsupportedCapability, result.Failure?.Category);
        Assert.Equal(0, provider.GenerationCalls);
    }

    private sealed class RejectingPolicy : IAiFeaturePolicy
    {
        public ValueTask<AiPolicyDecision> EvaluateAsync(AiTextRequest request, CancellationToken cancellationToken) => ValueTask.FromResult(AiPolicyDecision.Reject(AiFailureCategory.QuotaExceeded, "quota exceeded"));
    }

    private sealed class CapabilityLimitedProvider(IAiTextProvider inner) : IAiTextProvider
    {
        public AiProviderName Name => inner.Name;
        public AiProviderCapabilities Capabilities { get; } = new(new HashSet<string> { "text" });
        public ValueTask<AiGenerationResult> GenerateAsync(AiTextRequest request, CancellationToken cancellationToken = default) => inner.GenerateAsync(request, cancellationToken);
        public IAsyncEnumerable<AiStreamChunk> StreamAsync(AiTextRequest request, CancellationToken cancellationToken = default) => inner.StreamAsync(request, cancellationToken);
    }
}
