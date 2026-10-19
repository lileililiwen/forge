using Platform.Ai.Anthropic;
using Platform.Ai.Contracts;
using Platform.Ai.Ollama;
using Platform.Ai.OpenAiCompatible;

namespace Platform.Ai.Adapter.Tests;

public sealed class AdapterContractTests
{
    [Fact]
    public void Adapters_expose_explicit_provider_capabilities()
    {
        Assert.Contains("structured-output", new OpenAiCompatibleOptions().Capabilities);
        Assert.DoesNotContain("embeddings", new AnthropicOptions().Capabilities);
        Assert.Contains("streaming", new OllamaOptions().Capabilities);
    }

    [Fact]
    public async Task Cloud_adapters_fail_safely_when_not_configured()
    {
        var request = new AiTextRequest(AiFeatureKey.Create("draft"), [new("user", "secret")]);
        var openAi = await new OpenAiCompatibleProvider(new HttpClient()).GenerateAsync(request);
        var anthropic = await new AnthropicProvider(new HttpClient()).GenerateAsync(request);
        Assert.Equal(AiFailureCategory.Authentication, openAi.Failure?.Category);
        Assert.Equal(AiFailureCategory.Authentication, anthropic.Failure?.Category);
        Assert.Null(openAi.Text);
        Assert.Null(anthropic.Text);
    }
}
