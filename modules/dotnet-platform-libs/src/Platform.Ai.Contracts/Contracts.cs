#pragma warning disable CS1591

namespace Platform.Ai.Contracts;

public readonly record struct AiFeatureKey(string Value)
{
    public static AiFeatureKey Create(string value) => new(Require(value, nameof(value)));
    public override string ToString() => Value;
    private static string Require(string value, string name) => string.IsNullOrWhiteSpace(value) ? throw new ArgumentException("A non-empty value is required.", name) : value.Trim();
}

public readonly record struct AiProviderName(string Value)
{
    public static AiProviderName Create(string value) => new(string.IsNullOrWhiteSpace(value) ? throw new ArgumentException("A non-empty value is required.", nameof(value)) : value.Trim());
    public override string ToString() => Value;
}

public sealed record AiMessage
{
    public AiMessage(string role, string content)
    {
        Role = string.IsNullOrWhiteSpace(role) ? throw new ArgumentException("A non-empty value is required.", nameof(role)) : role;
        Content = string.IsNullOrWhiteSpace(content) ? throw new ArgumentException("A non-empty value is required.", nameof(content)) : content;
    }
    public string Role { get; }
    public string Content { get; }
}

public sealed record AiTextRequest
{
    public AiTextRequest(AiFeatureKey feature, IReadOnlyList<AiMessage> messages, string? model = null, int? maxOutputTokens = null, TimeSpan? timeout = null, IReadOnlyDictionary<string, string>? metadata = null)
    {
        Feature = feature;
        Messages = messages ?? throw new ArgumentNullException(nameof(messages));
        if (Messages.Count == 0) throw new ArgumentException("At least one message is required.", nameof(messages));
        if (maxOutputTokens is <= 0) throw new ArgumentOutOfRangeException(nameof(maxOutputTokens));
        if (timeout.HasValue && timeout.Value <= TimeSpan.Zero) throw new ArgumentOutOfRangeException(nameof(timeout));
        Model = model; MaxOutputTokens = maxOutputTokens; Timeout = timeout; Metadata = metadata;
    }
    public AiFeatureKey Feature { get; init; }
    public IReadOnlyList<AiMessage> Messages { get; init; }
    public string? Model { get; init; }
    public int? MaxOutputTokens { get; init; }
    public TimeSpan? Timeout { get; init; }
    public IReadOnlyDictionary<string, string>? Metadata { get; init; }
}

public sealed record AiStructuredRequest(AiTextRequest Text, string SchemaJson);
public sealed record AiEmbeddingRequest(AiFeatureKey Feature, IReadOnlyList<string> Inputs, string? Model = null, TimeSpan? Timeout = null);
public sealed record AiUsage(int InputTokens, int OutputTokens, int TotalTokens)
{
    public static AiUsage Empty => new(0, 0, 0);
}
public sealed record AiCost(decimal Amount, string Currency = "USD");
public sealed record AiCapabilityResult(bool Supported, string? Capability = null, string? Detail = null);
public sealed record AiEmbeddingResult(IReadOnlyList<IReadOnlyList<float>> Embeddings, AiUsage Usage, AiFailure? Failure = null);
public sealed record AiGenerationResult(string? Text, AiUsage Usage, AiCost? Cost = null, AiFailure? Failure = null, string? Model = null)
{
    public bool Succeeded => Failure is null;
}
public sealed record AiStreamChunk(string? Text, bool IsFinal, AiUsage? Usage = null, AiFailure? Failure = null);

public enum AiFailureCategory { Unknown, Unavailable, UnsupportedCapability, QuotaExceeded, Timeout, Authentication, RateLimited, InvalidRequest, ProviderError }
public sealed record AiFailure(AiFailureCategory Category, string Code, string Message, int? RetryAfterSeconds = null)
{
    public static AiFailure Safe(AiFailureCategory category, string message, string code = "ai.failure") => new(category, code, message);
}

public sealed record AiProviderCapabilities(IReadOnlySet<string> Names)
{
    public bool Supports(string capability) => Names.Any(item => item.Equals(capability, StringComparison.OrdinalIgnoreCase));
}

public interface IAiTextProvider
{
    AiProviderName Name { get; }
    AiProviderCapabilities Capabilities { get; }
    ValueTask<AiGenerationResult> GenerateAsync(AiTextRequest request, CancellationToken cancellationToken = default);
    IAsyncEnumerable<AiStreamChunk> StreamAsync(AiTextRequest request, CancellationToken cancellationToken = default);
}

public interface IAiEmbeddingProvider
{
    AiProviderName Name { get; }
    ValueTask<AiEmbeddingResult> EmbedAsync(AiEmbeddingRequest request, CancellationToken cancellationToken = default);
}

public interface IAiProviderRouter
{
    IAiTextProvider Resolve(AiFeatureKey feature);
}

public interface IAiFeaturePolicy
{
    ValueTask<AiPolicyDecision> EvaluateAsync(AiTextRequest request, CancellationToken cancellationToken = default);
}

public sealed record AiPolicyDecision(bool Allowed, AiFailure? Failure = null)
{
    public static AiPolicyDecision Allow() => new(true);
    public static AiPolicyDecision Reject(AiFailureCategory category, string message) => new(false, AiFailure.Safe(category, message));
}

public interface IAiUsageSink
{
    ValueTask RecordAsync(AiFeatureKey feature, AiUsage usage, CancellationToken cancellationToken = default);
}

public interface IAiTelemetry
{
    void Completed(AiFeatureKey feature, AiProviderName provider, string? model, TimeSpan latency, AiUsage usage);
    void Failed(AiFeatureKey feature, AiProviderName provider, AiFailure failure, TimeSpan latency);
}
