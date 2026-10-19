#pragma warning disable CS1591
using System.Net.Http.Json;
using System.Text.Json;
using Platform.Ai.Contracts;

namespace Platform.Ai.Anthropic;

public sealed class AnthropicOptions
{
    public Uri BaseAddress { get; set; } = new("https://api.anthropic.com/v1/");
    public string? ApiKey { get; set; }
    public HashSet<string> Capabilities { get; } = new(StringComparer.OrdinalIgnoreCase) { "text", "streaming" };
}
public sealed class AnthropicProvider : IAiTextProvider
{
    private readonly HttpClient _http; private readonly AnthropicOptions _options;
    public AnthropicProvider(HttpClient http, AnthropicOptions? options = null) { _http = http ?? throw new ArgumentNullException(nameof(http)); _options = options ?? new(); }
    public AiProviderName Name => AiProviderName.Create("anthropic");
    public AiProviderCapabilities Capabilities => new(_options.Capabilities);
    public async ValueTask<AiGenerationResult> GenerateAsync(AiTextRequest request, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(_options.ApiKey)) return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.Authentication, "The AI provider is not configured."));
        using var message = new HttpRequestMessage(HttpMethod.Post, new Uri(_options.BaseAddress, "messages"));
        message.Headers.TryAddWithoutValidation("x-api-key", _options.ApiKey); message.Headers.TryAddWithoutValidation("anthropic-version", "2023-06-01");
        message.Content = JsonContent.Create(new { model = request.Model ?? "default", max_tokens = request.MaxOutputTokens ?? 1024, messages = request.Messages.Where(m => m.Role != "system").Select(m => new { role = m.Role, content = m.Content }) });
        try
        {
            using var response = await _http.SendAsync(message, cancellationToken).ConfigureAwait(false);
            if (!response.IsSuccessStatusCode) return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.ProviderError, "The AI provider rejected the request."));
            using var document = JsonDocument.Parse(await response.Content.ReadAsStreamAsync(cancellationToken).ConfigureAwait(false));
            var text = document.RootElement.GetProperty("content")[0].GetProperty("text").GetString();
            return new(text, AiUsage.Empty, Model: request.Model);
        }
        catch (JsonException) { return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.ProviderError, "The AI provider returned an invalid response.")); }
        catch (HttpRequestException) { return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.Unavailable, "The AI provider is unavailable.")); }
    }
    public async IAsyncEnumerable<AiStreamChunk> StreamAsync(AiTextRequest request, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken cancellationToken = default) { yield return new AiStreamChunk((await GenerateAsync(request, cancellationToken).ConfigureAwait(false)).Text, true); }
}
