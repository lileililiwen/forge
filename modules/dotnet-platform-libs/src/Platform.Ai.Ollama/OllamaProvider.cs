#pragma warning disable CS1591
using System.Net.Http.Json;
using System.Text.Json;
using Platform.Ai.Contracts;

namespace Platform.Ai.Ollama;

public sealed class OllamaOptions
{
    public Uri BaseAddress { get; set; } = new("http://localhost:11434/api/");
    public HashSet<string> Capabilities { get; } = new(StringComparer.OrdinalIgnoreCase) { "text", "streaming", "embeddings" };
}
public sealed class OllamaProvider : IAiTextProvider
{
    private readonly HttpClient _http; private readonly OllamaOptions _options;
    public OllamaProvider(HttpClient http, OllamaOptions? options = null) { _http = http ?? throw new ArgumentNullException(nameof(http)); _options = options ?? new(); }
    public AiProviderName Name => AiProviderName.Create("ollama");
    public AiProviderCapabilities Capabilities => new(_options.Capabilities);
    public async ValueTask<AiGenerationResult> GenerateAsync(AiTextRequest request, CancellationToken cancellationToken = default)
    {
        using var response = await _http.PostAsJsonAsync(new Uri(_options.BaseAddress, "chat"), new { model = request.Model ?? "default", messages = request.Messages, stream = false }, cancellationToken).ConfigureAwait(false);
        if (!response.IsSuccessStatusCode) return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.Unavailable, "The local AI provider is unavailable."));
        try { using var document = JsonDocument.Parse(await response.Content.ReadAsStreamAsync(cancellationToken).ConfigureAwait(false)); return new(document.RootElement.GetProperty("message").GetProperty("content").GetString(), AiUsage.Empty, Model: request.Model); }
        catch (JsonException) { return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.ProviderError, "The AI provider returned an invalid response.")); }
    }
    public async IAsyncEnumerable<AiStreamChunk> StreamAsync(AiTextRequest request, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken cancellationToken = default) { yield return new AiStreamChunk((await GenerateAsync(request, cancellationToken).ConfigureAwait(false)).Text, true); }
}
