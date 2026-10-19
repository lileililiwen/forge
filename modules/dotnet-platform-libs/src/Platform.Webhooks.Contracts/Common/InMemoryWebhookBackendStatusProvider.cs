using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Common;

/// <summary>Default backend status implementation that reports the configured options and provider type.</summary>
public sealed class InMemoryWebhookBackendStatusProvider : IWebhookBackendStatusProvider
{
    private readonly WebhookOptions _options;

    /// <summary>Creates the default status provider.</summary>
    public InMemoryWebhookBackendStatusProvider(WebhookOptions options)
    {
        _options = options ?? throw new ArgumentNullException(nameof(options));
    }

    /// <inheritdoc />
    public WebhookBackendStatus GetStatus() => new("memory", Available: true, Detail: _options.TargetAllowList.Count > 0 ? "allow_list_configured" : null);
}
