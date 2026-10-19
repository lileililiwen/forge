#pragma warning disable CS1591
using System.Net.Http.Json;
using System.Text.Json;
using Platform.Ai.Contracts;

namespace Platform.Ai.OpenAiCompatible;

public sealed class OpenAiCompatibleOptions
{
    public Uri BaseAddress { get; set; } = new("https://api.openai.com/v1/");
    public string? ApiKey { get; set; }
    public string Provider { get; set; } = "openai";
    public HashSet<string> Capabilities { get; } = new(StringComparer.OrdinalIgnoreCase) { "text", "streaming", "structured-output", "embeddings" };
}

public class OpenAiCompatibleProvider : IAiTextProvider
{
    private readonly HttpClient _http;
    private readonly OpenAiCompatibleOptions _options;
    public OpenAiCompatibleProvider(HttpClient http, OpenAiCompatibleOptions? options = null) { _http = http ?? throw new ArgumentNullException(nameof(http)); _options = options ?? new(); }
    public AiProviderName Name => AiProviderName.Create(_options.Provider);
    public AiProviderCapabilities Capabilities => new(_options.Capabilities);
    public async ValueTask<AiGenerationResult> GenerateAsync(AiTextRequest request, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(_options.ApiKey)) return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.Authentication, "The AI provider is not configured."));
        using var message = new HttpRequestMessage(HttpMethod.Post, new Uri(_options.BaseAddress, "chat/completions"));
        message.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", _options.ApiKey);
        message.Content = JsonContent.Create(new { model = request.Model ?? "default", messages = request.Messages.Select(m => new { role = m.Role, content = m.Content }), max_tokens = request.MaxOutputTokens });
        try
        {
            using var response = await _http.SendAsync(message, cancellationToken).ConfigureAwait(false);
            if (!response.IsSuccessStatusCode) return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.ProviderError, "The AI provider rejected the request."));
            using var document = JsonDocument.Parse(await response.Content.ReadAsStreamAsync(cancellationToken).ConfigureAwait(false));
            var text = document.RootElement.GetProperty("choices")[0].GetProperty("message").GetProperty("content").GetString();
            return new(text, AiUsage.Empty, Model: request.Model);
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested) { return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.Timeout, "The AI request timed out.")); }
        catch (JsonException) { return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.ProviderError, "The AI provider returned an invalid response.")); }
        catch (HttpRequestException) { return new(null, AiUsage.Empty, Failure: AiFailure.Safe(AiFailureCategory.Unavailable, "The AI provider is unavailable.")); }
    }
    public async IAsyncEnumerable<AiStreamChunk> StreamAsync(AiTextRequest request, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken cancellationToken = default)
    { yield return new AiStreamChunk((await GenerateAsync(request, cancellationToken).ConfigureAwait(false)).Text, true); }
}

public sealed class DeepSeekProvider : OpenAiCompatibleProvider
{
    public DeepSeekProvider(HttpClient http, OpenAiCompatibleOptions? options = null) : base(http, options ?? new OpenAiCompatibleOptions { BaseAddress = new Uri("https://api.deepseek.com/v1/"), Provider = "deepseek" }) { }
}
